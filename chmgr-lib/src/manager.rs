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
use std::cell::{RefCell, RefMut};
use typesv4::prelude::*;

pub struct ManagerState {
    imax: u32,
    authenticated: bool,
}

impl ManagerState {
    pub fn default() -> Self {
        ManagerState {
            imax: 0,
            authenticated: false,
        }
    }
}

pub struct ManagerHandle {
    data_set: RefCell<ManagerState>,
    auth_api: &'static str,
    iec_api: &'static str,
    engy_api: &'static str,
}

impl ManagerHandle {
    pub fn new(auth_api: &'static str, iec_api: &'static str, engy_api: &'static str) -> &'static mut Self {
        let handle = ManagerHandle {
            auth_api,
            iec_api,
            engy_api,
            data_set: RefCell::new(ManagerState::default()),
        };

        // return a static handle to prevent Rust from complaining when moving/sharing it
        Box::leak(Box::new(handle))
    }

    #[track_caller]
    fn get_state(&self) -> Result<RefMut<'_, ManagerState>, AfbError> {
        match self.data_set.try_borrow_mut() {
            Err(_) => return afb_error!("charging-manager-update", "fail to access &mut data_set"),
            Ok(value) => Ok(value),
        }
    }

    pub fn slac(&self, evt: &AfbEventMsg, msg: &SlacStatus) -> Result<(), AfbError> {
        let mut data_set = self.get_state()?;

        match msg {
            SlacStatus::MATCHED => { /* start ISO15118 */ }
            SlacStatus::UNMATCHED | SlacStatus::TIMEOUT => {
                // roll back to NFC authentication
                afb_log_msg!(Notice, evt, "Requesting NFC get_contract");
                // if auth check is ok then allow power
                AfbSubCall::call_sync(evt.get_api(), self.auth_api, "get-contract", AFB_NO_DATA)?;
                data_set.authenticated = true;
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
                data_set.authenticated = true;
            }

            _ => {}
        }

        Ok(())
    }

    pub fn iec(&self, evt: &AfbEventMsg, msg: &Iec6185Msg) -> Result<(), AfbError> {
        let mut data_set = self.get_state()?;

        match msg {
            Iec6185Msg::PowerRqt(value) => {
                data_set.imax = *value;
                if data_set.authenticated {
                    AfbSubCall::call_sync(evt.get_api(), self.iec_api, "power", true)?;
                }
            }
            Iec6185Msg::Error(_value) => {
                data_set.imax = 0;
            }
            Iec6185Msg::RelayOn(_value) => {}
            Iec6185Msg::Plugged(value) => {
                if *value {
                   AfbSubCall::call_sync(evt.get_api(), self.engy_api, "Energy-Session", EnergyAction::RESET)?;
                }
            }
        }
        Ok(())
    }
}
