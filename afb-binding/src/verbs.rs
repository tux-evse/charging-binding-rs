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
use charging::prelude::*;
use typesv4::prelude::*;

struct OcppEvtCtx {
    mgr: &'static ManagerHandle,
}

fn ocpp_event_cb(evt: &AfbEventMsg, args: &AfbRqtData, ctx: &AfbCtxData) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<OcppEvtCtx>()?;

    let msg = args.get::<&OcppMsg>(0)?;

    // forward ocpp events to potential listeners
    afb_log_msg!(Debug, evt, "ocpp_evt:{:?}", msg);
    ctx.mgr.ocpp(evt, msg)?;

    Ok(())
}

struct IgnoreCtx {}

// Fulup TBD handle broadcast energy event
fn engy_ignore_cb(
    _evt: &AfbEventMsg,
    _args: &AfbRqtData,
    _ctx: &AfbCtxData,
) -> Result<(), AfbError> {
    let _ctx = _ctx.get_ref::<IgnoreCtx>()?;

    Ok(())
}

struct EngyIoverCtx {
    mgr: &'static ManagerHandle,
}

fn engy_iover_cb(evt: &AfbEventMsg, args: &AfbRqtData, ctx: &AfbCtxData) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<EngyIoverCtx>()?;

    let msg = args.get::<&MeterDataSet>(0)?;

    // forward engy events to potential listeners
    afb_log_msg!(Debug, evt, "engy_iover:{:?}", msg);
    ctx.mgr.engy_iover(evt, msg)?;

    Ok(())
}

struct EngyIavailCtx {
    mgr: &'static ManagerHandle,
}

fn engy_iavail_cb(evt: &AfbEventMsg, args: &AfbRqtData, ctx: &AfbCtxData) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<EngyIavailCtx>()?;

    let msg = args.get::<u32>(0)?;

    // forward engy events to potential listeners
    afb_log_msg!(Debug, evt, "engy_iavail:{:?}", msg);
    ctx.mgr.engy_imax(evt, msg)?;

    Ok(())
}

struct SlacEvtCtx {
    mgr: &'static ManagerHandle,
}

fn slac_event_cb(evt: &AfbEventMsg, args: &AfbRqtData, ctx: &AfbCtxData) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<SlacEvtCtx>()?;

    let msg = args.get::<&SlacStatus>(0)?;

    // forward slac events to potential listeners
    afb_log_msg!(Debug, evt, "slac_evt:{:?}", msg);
    ctx.mgr.slac(evt, msg)?;

    Ok(())
}

struct IecEvtCtx {
    mgr: &'static ManagerHandle,
}

fn iec_event_cb(evt: &AfbEventMsg, args: &AfbRqtData, ctx: &AfbCtxData) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<IecEvtCtx>()?;

    let msg = args.get::<&Iec6185Msg>(0)?;

    afb_log_msg!(Debug, evt, "iec_evt:{:?}", msg.clone());
    ctx.mgr.iec(evt, msg)?;

    Ok(())
}

struct SubscribeCtx {
    event: &'static AfbEvent,
}

fn subscribe_callback(
    request: &AfbRequest,
    args: &AfbRqtData,
    ctx: &AfbCtxData,
) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<SubscribeCtx>()?;

    let subcription = args.get::<bool>(0)?;
    if subcription {
        ctx.event.subscribe(request)?;
    } else {
        ctx.event.unsubscribe(request)?;
    }
    request.reply(AFB_NO_DATA, 0);
    Ok(())
}

struct StateRequestCtx {
    mgr: &'static ManagerHandle,
    evt: &'static AfbEvent,
}

fn state_request_cb(rqt: &AfbRequest, args: &AfbRqtData, ctx: &AfbCtxData) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<StateRequestCtx>()?;

    match args.get::<&ChargingAction>(0)? {
        ChargingAction::READ => {
            let data_set = ctx.mgr.get_state()?;
            rqt.reply(data_set.clone(), 0);
        }

        ChargingAction::SUBSCRIBE => {
            afb_log_msg!(Notice, rqt, "Subscribe {}", ctx.evt.get_uid());
            ctx.evt.subscribe(rqt)?;
            rqt.reply(AFB_NO_DATA, 0);
        }

        ChargingAction::UNSUBSCRIBE => {
            afb_log_msg!(Notice, rqt, "Unsubscribe {}", ctx.evt.get_uid());
            ctx.evt.unsubscribe(rqt)?;
            rqt.reply(AFB_NO_DATA, 0);
        }
    }
    Ok(())
}

struct ReserveChargerCtx {
    mgr: &'static ManagerHandle,
}

fn reserve_charger_cb(
    rqt: &AfbRequest,
    args: &AfbRqtData,
    ctx: &AfbCtxData,
) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<ReserveChargerCtx>()?;

    let reservation = args.get::<&ReservationSession>(0)?;
    let status = ctx.mgr.reserve(&reservation)?;
    rqt.reply(status, 0);

    Ok(())
}

struct RemotePowerData {
    mgr: &'static ManagerHandle,
}

fn remote_power_callback(
    request: &AfbRequest,
    args: &AfbRqtData,
    ctx: &AfbCtxData,
) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<RemotePowerData>()?;

    let enable = args.get::<bool>(0)?;

    ctx.mgr.powerctrl(enable)?;

    request.reply(AFB_NO_DATA, 0);
    Ok(())
}

struct TimerCtx {
    mgr: &'static ManagerHandle,
    evt: &'static AfbEvent,
}
// send charging state every tic ms.
fn timer_callback(_timer: &AfbTimer, _decount: u32, ctx: &AfbCtxData) -> Result<(), AfbError> {
    let ctx = ctx.get_ref::<TimerCtx>()?;

    let state = ctx.mgr.get_state()?;
    ctx.evt.push(state.clone());
    Ok(())
}

pub(crate) fn register_verbs(
    apiv4: AfbApiV4,
    api: &mut AfbApi,
    config: BindingCfg,
) -> Result<(), AfbError> {
    let msg_evt = AfbEvent::new("msg");
    let manager = ManagerHandle::new(
        apiv4,
        config.auth_api,
        config.iec_api,
        config.engy_api,
        config.ocpp_api,
        msg_evt,
        config.basic_charging_enabled,
    );

    let state_event = AfbEvent::new("state");
    if config.tic > 0 {
        AfbTimer::new("tic-timer")
            .set_period(config.tic)
            .set_decount(0)
            .set_callback(timer_callback)
            .set_context(TimerCtx {
                mgr: manager,
                evt: state_event,
            })
            .start()?;
    }

    let state_verb = AfbVerb::new("charging-state")
        .set_name("state")
        .set_info("current charging state")
        .set_actions("['read','subscribe','unsubscribe']")?
        .set_callback(state_request_cb)
        .set_context(StateRequestCtx {
            mgr: manager,
            evt: state_event,
        })
        .finalize()?;

    let reserve_verb = AfbVerb::new("reserve-charger")
        .set_name("reserve")
        .set_info("reserve charging station start/stop data")
        .set_actions("['now','delay','cancel']")?
        .set_callback(reserve_charger_cb)
        .set_context(ReserveChargerCtx { mgr: manager })
        .finalize()?;

    let subscribe_verb = AfbVerb::new("subscribe-msg")
        .set_name("subscribe")
        .set_callback(subscribe_callback)
        .set_context(SubscribeCtx { event: msg_evt })
        .set_info("subscribe charging events")
        .set_usage("true|false")
        .finalize()?;

    let iec_handler = AfbEvtHandler::new("iec-evt")
        .set_pattern(to_static_str(format!("{}/*", config.iec_api)))
        .set_callback(iec_event_cb)
        .set_context(IecEvtCtx { mgr: manager })
        .finalize()?;

    let slac_handler = AfbEvtHandler::new("slac-evt")
        .set_pattern(to_static_str(format!("{}/*", config.slac_api)))
        .set_callback(slac_event_cb)
        .set_context(SlacEvtCtx { mgr: manager })
        .finalize()?;

    let iover_handler = AfbEvtHandler::new("iover-evt")
        .set_pattern(to_static_str(format!("{}/iover", config.engy_api)))
        .set_callback(engy_iover_cb)
        .set_context(EngyIoverCtx { mgr: manager })
        .finalize()?;

    let ignore_handler = AfbEvtHandler::new("over-limit")
        .set_pattern(to_static_str(format!("{}/over-limit", config.engy_api)))
        .set_callback(engy_ignore_cb)
        .set_context(IgnoreCtx {})
        .finalize()?;

    let iavail_handler = AfbEvtHandler::new("iavail-evt")
        .set_pattern(to_static_str(format!("{}/iavail", config.engy_api)))
        .set_callback(engy_iavail_cb)
        .set_context(EngyIavailCtx { mgr: manager })
        .finalize()?;

    let remote_power_verb = AfbVerb::new("remote_power")
        .set_callback(remote_power_callback)
        .set_context(RemotePowerData { mgr: manager })
        .set_info("allow power (true/false)")
        .set_usage("true/false")
        .finalize()?;

    api.add_evt_handler(iover_handler);
    api.add_evt_handler(iavail_handler);
    api.add_evt_handler(iec_handler);
    api.add_evt_handler(slac_handler);
    api.add_evt_handler(ignore_handler);

    if config.ocpp_api.is_some() {
        let ocpp_handler = AfbEvtHandler::new("ocpp-evt")
            .set_pattern(to_static_str(format!("{}/*", config.ocpp_api.unwrap())))
            .set_callback(ocpp_event_cb)
            .set_context(OcppEvtCtx { mgr: manager })
            .finalize()?;

        api.add_evt_handler(ocpp_handler);
    }
    api.add_event(msg_evt);
    api.add_event(state_event);
    api.add_verb(state_verb);
    api.add_verb(reserve_verb);
    api.add_verb(subscribe_verb);

    api.add_verb(remote_power_verb);

    Ok(())
}
