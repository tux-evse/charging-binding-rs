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

use crate::prelude::*;
use afbv4::prelude::*;
use typesv4::prelude::*;

pub struct BindingCfg {
    pub iec_api: &'static str,
    pub slac_api: &'static str,
    pub auth_api: &'static str,
    pub engy_api: &'static str,
    pub ocpp_api: &'static str,
    pub tic: u32,
    pub limit: u32,
}

pub struct ApiUserData {
    pub iec_api: &'static str,
    pub slac_api: &'static str,
    pub engy_api: &'static str,
    pub ocpp_api: &'static str,
}

impl AfbApiControls for ApiUserData {
    // the API is created and ready. At this level user may subcall api(s) declare as dependencies
    fn start(&mut self, api: &AfbApi) -> Result<(), AfbError> {
        AfbSubCall::call_sync(api, self.iec_api, "subscribe", true)?;
        AfbSubCall::call_sync(api, self.slac_api, "subscribe", true)?;
        AfbSubCall::call_sync(api, self.ocpp_api, "subscribe", true)?;
        AfbSubCall::call_sync(api, self.engy_api, "iavail", EnergyAction::SUBSCRIBE)?;
        AfbSubCall::call_sync(api, self.engy_api, "iover", EnergyAction::SUBSCRIBE)?;
        Ok(())
    }

    // mandatory unsed declaration
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

// Binding init callback started at binding load time before any API exist
// -----------------------------------------
pub fn binding_init(rootv4: AfbApiV4, jconf: JsoncObj) -> Result<&'static AfbApi, AfbError> {
    afb_log_msg!(Info, rootv4, "config:{}", jconf);

    // add binding custom converter
    chmgr_registers()?;
    am62x_registers()?;
    slac_registers()?;
    engy_registers()?;
    auth_registers()?;
    ocpp_registers()?;

    let uid = if let Ok(value) = jconf.get::<String>("uid") {
        to_static_str(value)
    } else {
        "chmgr"
    };

    let api = if let Ok(value) = jconf.get::<String>("api") {
        to_static_str(value)
    } else {
        uid
    };

    let info = if let Ok(value) = jconf.get::<String>("info") {
        to_static_str(value)
    } else {
        ""
    };

    let limit = if let Ok(value) = jconf.get::<u32>("climit") {
        value
    } else {
        64
    };

    let iec_api = jconf.get::<&'static str>("iec_api")?;
    let slac_api = jconf.get::<&'static str>("slac_api")?;
    let auth_api = jconf.get::<&'static str>("auth_api")?;
    let engy_api = jconf.get::<&'static str>("energy_api")?;
    let ocpp_api = jconf.get::<&'static str>("ocpp_api")?;
    let tic = jconf.default::<u32>("tic", 0)?;
    let config = BindingCfg {
        iec_api,
        slac_api,
        auth_api,
        engy_api,
        ocpp_api,
        tic,
        limit,
    };

    // create backend API
    let api = AfbApi::new(api)
        .set_info(info)
        .require_api(iec_api)
        .require_api(slac_api)
        .require_api(engy_api)
        .require_api(auth_api)
        .require_api(ocpp_api)
        .set_callback(Box::new(ApiUserData {
            iec_api,
            slac_api,
            engy_api,
            ocpp_api,
        }));

    if let Ok(value) = jconf.get::<String>("permission") {
        api.set_permission(AfbPermission::new(to_static_str(value)));
    };

    if let Ok(value) = jconf.get::<i32>("verbosity") {
        api.set_verbosity(value);
    };

    register_verbs(rootv4, api, config)?;
    Ok(api.finalize()?)
}

// register binding within libafb
AfbBindingRegister!(binding_init);
