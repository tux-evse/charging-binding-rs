/*
 * Copyright (C) 2015-2022 IoT.bzh Company
 * Author: Fulup Ar Foll <fulup@iot.bzh>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 */

use afbv4::prelude::*;
use std::cell::{Ref, RefCell, RefMut};
use typesv4::prelude::*;

pub struct ManagerHandle {
    data_set: RefCell<ChargingState>,
    auth_api: &'static str,
    iec_api: &'static str,
    engy_api: &'static str,
    event: &'static AfbEvent,
}

impl ManagerHandle {
    pub fn new(
        auth_api: &'static str,
        iec_api: &'static str,
        engy_api: &'static str,
        event: &'static AfbEvent,
    ) -> &'static mut Self {
        let handle = ManagerHandle {
            auth_api,
            iec_api,
            engy_api,
            event,
            data_set: RefCell::new(ChargingState::default()),
        };

        // return a static handle to prevent Rust from complaining when moving/sharing it
        Box::leak(Box::new(handle))
    }

    #[track_caller]
    pub fn get_state(&self) -> Result<RefMut<'_, ChargingState>, AfbError> {
        match self.data_set.try_borrow_mut() {
            Err(_) => return afb_error!("charging-manager-update", "fail to access &mut data_set"),
            Ok(value) => Ok(value),
        }
    }

    #[track_caller]
    pub fn check_state(&self) -> Result<Ref<'_, ChargingState>, AfbError> {
        match self.data_set.try_borrow() {
            Err(_) => return afb_error!("charging-manager-update", "fail to access &mut data_set"),
            Ok(value) => Ok(value),
        }
    }

    // Fulup TBD reservation is far more complex and should rely on backend interaction
    pub fn reserve (&self, reservation: &ReservationSession) -> Result <ReservationStatus, AfbError> {
        let mut data_set= self.get_state()?;
        let response= match &data_set.reservation {
            None => {
                match reservation.status {
                    ReservationStatus::Request => {
                        let resa= ReservationState {
                            id: reservation.id,
                            start: reservation.start,
                            stop: reservation.stop,
                        };
                         data_set.reservation= Some(resa);
                        ReservationStatus::Accepted
                    }
                    _ => { return afb_error!("reservation-not-present", "current request:{:?}", reservation.status)}

                }
            }
            Some(value) =>  match reservation.status {
                ReservationStatus::Cancel => {

                    if value.id != reservation.id {
                        return afb_error!("reservation-invalid-id", "current session:{} request:{}", value.id, reservation.id)
                    }
                    data_set.reservation= None;
                    ReservationStatus::Cancel
                },
                _ => {
                    return afb_error!("reservation-already-running", "current session:{} request:{:?}",value.id, reservation.status)
                }
            }
        };

        self.event.push(ChargingMsg::Reservation(response));
        Ok(response)
    }

    pub fn push_state(&self) -> Result<(), AfbError> {
        let data_set = self.get_state()?;
        self.event.push(data_set.clone());
        Ok(())
    }

    fn nfc_auth(&self, evt: &AfbEventMsg) -> Result<(), AfbError> {
        {
            let mut data_set = self.get_state()?;
            afb_log_msg!(Notice, self.event, "Requesting nfc-auth");
            data_set.auth = AuthMsg::Pending;
            self.event.push(ChargingMsg::Auth(data_set.auth));
        }

        // Fulup TBD clean wait 5s to simulate a user action
        use std::{thread, time};
        thread::sleep(time::Duration::from_millis(5000));

        // if auth check is ok then allow power
        let mut data_set = self.get_state()?;
        match AfbSubCall::call_sync(evt.get_apiv4(), self.auth_api, "nfc-auth", AFB_NO_DATA) {
            Ok(response) => {
                let contract = response.get::<&AuthState>(0)?;
                data_set.auth = contract.auth;

                let response = AfbSubCall::call_sync(
                    evt.get_apiv4(),
                    self.engy_api,
                    "config",
                    EngyConfSet {
                        pmax: data_set.pmax as i32,
                        imax: data_set.imax as i32,
                    },
                )?;

                let engy_conf = response.get::<&EngyConfSet>(0)?;
                data_set.imax = engy_conf.imax as u32;
                data_set.pmax = engy_conf.pmax as u32;
                self.event.push(ChargingMsg::Auth(data_set.auth));
            }
            Err(_) => {
                data_set.auth = AuthMsg::Fail;
                self.event.push(ChargingMsg::Auth(data_set.auth));
                AfbSubCall::call_sync(evt.get_apiv4(), self.iec_api, "power", false)?;
                return afb_error!("charg-iec-auth", "fail to authenticate with NFC");
            }
        }

        // force firmware imax/pwm
        afb_log_msg!(Notice, self.event, "Valid nfc-auth");
        Ok(())
    }

    pub fn slac(&self, evt: &AfbEventMsg, msg: &SlacStatus) -> Result<(), AfbError> {
        match msg {
            SlacStatus::MATCHED => {
                /* start ISO15118 */
                AfbSubCall::call_sync(evt.get_apiv4(), self.iec_api, "power", true)?;
            }
            SlacStatus::UNMATCHED | SlacStatus::TIMEOUT => {
                self.event.push(ChargingMsg::Iso(IsoState::Iec));
                self.nfc_auth(evt)?;

                AfbSubCall::call_sync(evt.get_apiv4(), self.iec_api, "power", true)?;
                self.event.push(ChargingMsg::Power(PowerRequest::Start));
                afb_log_msg!(Notice, self.event, "set eic power:true");
            }

            _ => {}
        }

        Ok(())
    }

    pub fn engy(&self, evt: &AfbEventMsg, msg: &MeterDataSet) -> Result<(), AfbError> {
        let mut data_set = self.get_state()?;

        match msg.tag {
            MeterTagSet::OverCurrent => {
                // in current implementation overcurrent
                afb_log_msg!(Warning, evt, "energy over-current stop charge");
                AfbSubCall::call_sync(evt.get_api(), self.iec_api, "power", false)?;
                data_set.auth = AuthMsg::Idle;
            }

            _ => {}
        }

        Ok(())
    }

    pub fn iec(&self, evt: &AfbEventMsg, msg: &Iec6185Msg) -> Result<(), AfbError> {
        let mut data_set = self.get_state()?;

        match msg {
            Iec6185Msg::PowerRqt(value) => {
                afb_log_msg!(
                    Notice,
                    self.event,
                    "eic power-request value:{}", value);
            }
            Iec6185Msg::CableImax(value) => {
                afb_log_msg!(
                    Notice,
                    self.event,
                    "eic cable-imax new:{} old:{}",
                    value,
                    data_set.imax
                );
                data_set.imax=*value;
            }
            Iec6185Msg::Error(_value) => {
                data_set.imax = 0;
            }
            Iec6185Msg::RelayOn(value) => {
                if *value {
                    // vehicle start charging
                    AfbSubCall::call_sync(evt.get_apiv4(), self.iec_api, "imax", data_set.imax)?;
                    self.event
                        .push(ChargingMsg::Power(PowerRequest::Charging(data_set.imax)));
                } else {
                    // vehicle stop charging
                    let response = AfbSubCall::call_sync(
                        evt.get_api(),
                        self.engy_api,
                        "energy",
                        EnergyAction::READ,
                    )?;
                    let data = response.get::<&MeterDataSet>(0)?;
                    data_set.power = PowerRequest::Stop(data.total);
                    self.event
                        .push(ChargingMsg::Power(PowerRequest::Stop(data.total)));
                }
            }
            Iec6185Msg::Plugged(value) => {
                // reset authentication and energy session values
                AfbSubCall::call_sync(evt.get_api(), self.auth_api, "reset-auth", AFB_NO_DATA)?;
                AfbSubCall::call_sync(evt.get_api(), self.engy_api, "energy", EnergyAction::RESET)?;
                if *value {
                    self.event.push(ChargingMsg::Plugged(PlugState::Lock));
                } else {
                    self.event.push(ChargingMsg::Power(PowerRequest::Idle));
                    self.event.push(ChargingMsg::Plugged(PlugState::PlugOut));
                }
            }
        }
        Ok(())
    }
}
