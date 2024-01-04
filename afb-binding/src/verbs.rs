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
use typesv4::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

use crate::prelude::*;
// use libchmgr::prelude::*;

struct PlugStateCtx {
    vehicle: Rc<RefCell<VehicleState>>,
}
AfbVerbRegister!(PlugStateVerb, plug_status_cb, PlugStateCtx);
fn plug_status_cb(rqt: &AfbRequest, args: &AfbData, ctx: &mut PlugStateCtx) -> Result<(), AfbError> {
    let mut vehicle = match ctx.vehicle.try_borrow_mut() {
        Err(_) => return afb_error!("vehicle-state-update", "fail to access vehicle state",),
        Ok(value) => value,
    };

    // update vehicle state
    vehicle.plugged = *args.get::<&PlugState>(0)?;
    afb_log_msg!(Debug, rqt,"update plug status={:?}", vehicle.plugged);

    rqt.reply(AFB_NO_DATA, 0);
    Ok(())
}

struct PowerRequestCtx {
    vehicle: Rc<RefCell<VehicleState>>,
}
AfbVerbRegister!(PowerRequestVerb, power_request_cb, PowerRequestCtx);
fn power_request_cb(
    rqt: &AfbRequest,
    args: &AfbData,
    ctx: &mut PowerRequestCtx,
) -> Result<(), AfbError> {
    let mut vehicle = match ctx.vehicle.try_borrow_mut() {
        Err(_) => return afb_error!("vehicle-state-update", "fail to access vehicle state",),
        Ok(value) => value,
    };

    // update vehicle state
    vehicle.power_request = *args.get::<&PowerRequest>(0)?;
    afb_log_msg!(Debug, rqt,"update power_request={:?}", vehicle.power_request);

    rqt.reply(AFB_NO_DATA, 0);
    Ok(())
}
struct IecStateCtx {
    vehicle: Rc<RefCell<VehicleState>>,
}
AfbVerbRegister!(IecStateVerb, iec_state_cb, IecStateCtx);
fn iec_state_cb(
    rqt: &AfbRequest,
    args: &AfbData,
    ctx: &mut IecStateCtx,
) -> Result<(), AfbError> {
    let mut vehicle = match ctx.vehicle.try_borrow_mut() {
        Err(_) => return afb_error!("vehicle-state-update", "fail to access vehicle state",),
        Ok(value) => value,
    };

    // update vehicle state
    vehicle.iec_state = *args.get::<&IecState>(0)?;
    afb_log_msg!(Debug, rqt,"update iec state={:?}", vehicle.iec_state);

    rqt.reply(AFB_NO_DATA, 0);
    Ok(())
}

struct ImaxRequestCtx {
    vehicle: Rc<RefCell<VehicleState>>,
}
AfbVerbRegister!(ImaxRequestVerb, imax_request_cb, ImaxRequestCtx);
fn imax_request_cb(
    rqt: &AfbRequest,
    args: &AfbData,
    ctx: &mut ImaxRequestCtx,
) -> Result<(), AfbError> {
    let mut vehicle = match ctx.vehicle.try_borrow_mut() {
        Err(_) => return afb_error!("vehicle-state-update", "fail to access vehicle state",),
        Ok(value) => value,
    };

    // update vehicle state
    vehicle.power_imax = args.get::<u32>(0)?;
    afb_log_msg!(Debug, rqt,"update power imaxs={:?}", vehicle.power_imax);

    rqt.reply(AFB_NO_DATA, 0);
    Ok(())
}

pub(crate) fn register_verbs(api: &mut AfbApi, _config: BindingCfg) -> Result<(), AfbError> {
    // place vehicle state into a refcell to allow changes from non &mut pointer
    let vehicle = Rc::new(RefCell::new(VehicleState {
        plugged: PlugState::Unknown,
        power_request: PowerRequest::Stop,
        power_imax: 0,
        iso15118: Iso15118State::Unset,
        iec_state: IecState::Unset,
    }));

    let plug_status = AfbVerb::new("PlugState")
        .set_name("plug-state")
        .set_callback(Box::new(PlugStateCtx { vehicle: vehicle.clone() }))
        .set_info("Update ISO/IEC states")
        .set_action("['PLUG-LOCK','PLUG-ERR','PLUG-IDLE']")?
        .finalize()?;

    let power_rqt = AfbVerb::new("PowerRequest")
        .set_name("power-request")
        .set_callback(Box::new(PowerRequestCtx { vehicle: vehicle.clone() }))
        .set_info("Select power request mode")
        .set_action("['START','STOP']")?
        .finalize()?;

    let power_imax = AfbVerb::new("PowerImax")
        .set_name("power-imax")
        .set_callback(Box::new(ImaxRequestCtx { vehicle: vehicle.clone() }))
        .set_info("Update current imax")
        .set_usage("integer")
        .finalize()?;

    api.add_verb(plug_status);
    api.add_verb(power_rqt);
    api.add_verb(power_imax);

    Ok(())
}
