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
use std::sync::{Mutex, MutexGuard};
use typesv4::prelude::*;

pub struct ManagerHandle {
    apiv4: AfbApiV4,
    data_set: Mutex<ChargingState>,
    auth_api: &'static str,
    iec_api: &'static str,
    engy_api: &'static str,
    ocpp_api: &'static str,
    event: &'static AfbEvent,
}

AfbCallRegister!(IgnoreRspCtrl, ignore_rsp_cb);
fn ignore_rsp_cb(_api: &AfbApi, _args: &AfbData) -> Result<(), AfbError> {
    Ok(())
}

impl ManagerHandle {
    pub fn new(
        apiv4: AfbApiV4,
        auth_api: &'static str,
        iec_api: &'static str,
        engy_api: &'static str,
        ocpp_api: &'static str,
        event: &'static AfbEvent,
    ) -> &'static mut Self {
        let handle = ManagerHandle {
            apiv4,
            auth_api,
            iec_api,
            engy_api,
            ocpp_api,
            event,
            data_set: Mutex::new(ChargingState::default()),
        };

        // return a static handle to prevent Rust from complaining when moving/sharing it
        Box::leak(Box::new(handle))
    }

    #[track_caller]
    pub fn get_state(&self) -> Result<MutexGuard<'_, ChargingState>, AfbError> {
        let guard = self.data_set.lock().unwrap();
        Ok(guard)
    }

    // Fulup TBD reservation is far more complex and should rely on backend interaction
    pub fn reserve(&self, reservation: &ReservationSession) -> Result<ReservationStatus, AfbError> {
        let mut data_set = self.get_state()?;
        let response = match &data_set.reservation {
            None => match reservation.status {
                ReservationStatus::Request => {
                    let resa = ReservationState {
                        id: reservation.id,
                        start: reservation.start,
                        stop: reservation.stop,
                    };
                    data_set.reservation = Some(resa);
                    ReservationStatus::Accepted
                }
                _ => {
                    return afb_error!(
                        "reservation-not-present",
                        "current request:{:?}",
                        reservation.status
                    )
                }
            },
            Some(value) => match reservation.status {
                ReservationStatus::Cancel => {
                    if value.id != reservation.id {
                        return afb_error!(
                            "reservation-invalid-id",
                            "current session:{} request:{}",
                            value.id,
                            reservation.id
                        );
                    }
                    data_set.reservation = None;
                    ReservationStatus::Cancel
                }
                _ => {
                    return afb_error!(
                        "reservation-already-running",
                        "current session:{} request:{:?}",
                        value.id,
                        reservation.status
                    )
                }
            },
        };

        self.event.push(ChargingMsg::Reservation(response));
        Ok(response)
    }

    pub fn push_state(&self) -> Result<(), AfbError> {
        let data_set = self.get_state()?;
        self.event.push(data_set.clone());
        Ok(())
    }

    fn auth_rqt(
        &self,
        data_set: &mut MutexGuard<ChargingState>,
        evt: &AfbEventMsg,
    ) -> Result<(), AfbError> {
        afb_log_msg!(Notice, self.event, "Requesting idp-login");
        data_set.auth = AuthMsg::Pending;
        self.event.push(ChargingMsg::Auth(data_set.auth));

        match AfbSubCall::call_sync(evt.get_apiv4(), self.auth_api, "login", AFB_NO_DATA) {
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

                // set imax configuration
                AfbSubCall::call_async(evt.get_apiv4(), self.iec_api, "imax", data_set.imax, Box::new(IgnoreRspCtrl))?;
            }
            Err(_) => {
                data_set.auth = AuthMsg::Fail;
                self.event.push(ChargingMsg::Auth(data_set.auth));
                AfbSubCall::call_sync(evt.get_apiv4(), self.iec_api, "power", false)?;
                return afb_error!("charg-iec-auth", "fail idp authentication");
            }
        }

        data_set.auth = AuthMsg::Done;
        afb_log_msg!(Notice, self.event, "Valid idp-auth");
        Ok(())
    }

    pub fn slac(&self, evt: &AfbEventMsg, msg: &SlacStatus) -> Result<(), AfbError> {
        let mut state = self.get_state()?;
        let iso_state = match msg {
            SlacStatus::MATCHED => {
                /* start ISO15118 Fulup TBD should set imax */
                IsoState::Iso3
            }
            SlacStatus::UNMATCHED | SlacStatus::TIMEOUT => {
                self.auth_rqt(&mut state, evt)?; // Warning lock data_set
                IsoState::Iec
            }

            _ => {
                return Ok(()); /* silently ignore any other messages */
            }
        };
        state.iso = iso_state;
        AfbSubCall::call_async(evt.get_apiv4(), self.iec_api, "power", true, Box::new(IgnoreRspCtrl{}))?;
        self.event.push(ChargingMsg::Iso(iso_state));
        self.event.push(ChargingMsg::Power(PowerRequest::Start));
        afb_log_msg!(
            Notice,
            self.event,
            "Slac+Auth done allow power iso_mode:{:?}",
            iso_state
        );
        Ok(())
    }

    pub fn ocpp(&self, evt: &AfbEventMsg, msg: &OcppMsg) -> Result<(), AfbError> {
        let mut data_set = self.get_state()?;
        match msg {
            OcppMsg::PowerLimit(limit) => {
                // in current implementation over-current
                afb_log_msg!(Warning, evt, "ocpp set power limit:{}", limit.imax);
                if limit.imax < data_set.imax as i32 {
                    AfbSubCall::call_sync(evt.get_api(), self.iec_api, "imax", limit.imax)?;
                }
            }
            OcppMsg::Reservation(reservation) => {
                // in current implementation over-current
                afb_log_msg!(
                    Warning,
                    evt,
                    "ocpp reservation status:{:?}",
                    reservation.status
                );
                self.reserve(reservation)?;
            }
            OcppMsg::Reset => {
                // in current implementation over-current
                afb_log_msg!(Warning, evt, "ocpp reset power");
                AfbSubCall::call_sync(evt.get_api(), self.iec_api, "power", false)?;
            }

            OcppMsg::remote_stop_transaction (bool) => {
                // new event for re mote stop
                afb_log_msg!(Warning, evt, "ocpp stop received");
                AfbSubCall::call_sync(self.apiv4, self.iec_api, "power", false)?;
                data_set.power = PowerRequest::Idle;
            }

            _ => {}
        }
        Ok(())
    }

    pub fn engy_iover(&self, evt: &AfbEventMsg, msg: &MeterDataSet) -> Result<(), AfbError> {
        let mut data_set = self.get_state()?;

        match msg.tag {
            MeterTagSet::OverCurrent => {
                // in current implementation over-current
                afb_log_msg!(Warning, evt, "energy over-current stop charge");
                AfbSubCall::call_sync(evt.get_api(), self.iec_api, "power", false)?;
                data_set.power = PowerRequest::Idle;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn engy_imax(&self, evt: &AfbEventMsg, imax: u32) -> Result<(), AfbError> {
        let data_set = self.get_state()?;

        if let PowerRequest::Charging(current) = data_set.power {
            if current > imax {
                AfbSubCall::call_sync(evt.get_api(), self.iec_api, "imax", imax)?;
                self.event
                    .push(ChargingMsg::Power(PowerRequest::Charging(imax)));
            }
        }
        Ok(())
    }


// added for OCPP RemoteStopTransaction
pub fn powerctrl(&self, allow: bool) -> Result<(), AfbError> {
    let mut data_set = self.get_state()?;

    if allow {
        afb_log_msg!(Notice, None, "function remote power triggered, allow power");
        AfbSubCall::call_sync(self.apiv4, self.iec_api, "power", true)?;
    }
    else {
        afb_log_msg!(Notice, None, "function remote power triggered, stop power");
        AfbSubCall::call_sync(self.apiv4, self.iec_api, "power", false)?;
        data_set.power = PowerRequest::Idle;
    }

    Ok(())
}

    pub fn iec(&self, evt: &AfbEventMsg, msg: &Iec6185Msg) -> Result<(), AfbError> {
        let mut data_set = self.get_state()?;
        match msg {
            Iec6185Msg::PowerRqt(value) => {
                afb_log_msg!(Notice, self.event, "eic power-request value:{}", value);
                let status = {
                    if *value {
                        // B => C
                        data_set.plugged = PlugState::Lock;
                    } else {
                        // C => B

                        data_set.plugged = PlugState::PlugOut;
                    }
                    data_set.plugged
                };
                self.event.push(ChargingMsg::Plugged(status));
            }
            Iec6185Msg::CableImax(value) => {
                afb_log_msg!(
                    Notice,
                    self.event,
                    "eic cable-imax new:{} old:{}",
                    value,
                    data_set.imax
                );
                data_set.imax = *value;
            }
            Iec6185Msg::Error(_value) => {
                data_set.imax = 0;
            }
            Iec6185Msg::RelayOn(value) => {
                if *value {
                    // vehicle start charging
                    data_set.power = PowerRequest::Charging(data_set.imax);
                    AfbSubCall::call_sync(evt.get_apiv4(), self.iec_api, "imax", data_set.imax)?;
                    AfbSubCall::call_sync(
                        evt.get_apiv4(),
                        self.ocpp_api,
                        "status-notification",
                        OcppChargerStatus::Charging,
                    )?;
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
                    data_set.plugged = PlugState::PlugOut;
                }
                self.event.push(ChargingMsg::Power(data_set.power));
            }
            Iec6185Msg::Plugged(value) => {
                // reset authentication and energy session values
                let response = AfbSubCall::call_sync(
                    evt.get_api(),
                    self.engy_api,
                    "energy",
                    EnergyAction::RESET,
                )?;
                let data = response.get::<&MeterDataSet>(0)?;

                let plug_state = if *value {
                    data_set.plugged = PlugState::Lock;
                    AfbSubCall::call_sync(
                        evt.get_apiv4(),
                        self.ocpp_api,
                        "status-notification",
                        OcppChargerStatus::Reserved,
                    )?;
                    PlugState::Lock
                } else {
                    afb_log_msg!(
                        Debug,
                        self.event,
                        "Logout notification api:{}/logout total:{}",
                        self.auth_api,
                        data.total
                    );
                    let power = {
                        data_set.plugged = PlugState::PlugOut;
                        data_set.power = PowerRequest::Idle;
                        data_set.power
                    };
                    self.event.push(ChargingMsg::Power(power));
                    AfbSubCall::call_sync(evt.get_api(), self.auth_api, "logout", data.total)?;
                    PlugState::PlugOut
                };
                self.event.push(ChargingMsg::Plugged(plug_state));
            }
        }
        Ok(())
    }
}
