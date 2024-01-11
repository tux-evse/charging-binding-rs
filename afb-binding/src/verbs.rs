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

struct EngyEvtCtx {
    evt: &'static AfbEvent,
    mgr: &'static ManagerHandle,
}
AfbEventRegister!(EngyEvtCtrl, engy_event_cb, EngyEvtCtx);
fn engy_event_cb(evt: &AfbEventMsg, args: &AfbData, ctx: &mut EngyEvtCtx) -> Result<(), AfbError> {
    let msg = args.get::<&MeterDataSet>(0)?;

    // forward engy events to potential listeners
    afb_log_msg!(Debug, evt, "engy_evt:{:?}", msg);
    ctx.evt.push(msg.clone());
    ctx.mgr.engy(evt, msg)?;

    Ok(())
}

struct SlacEvtCtx {
    evt: &'static AfbEvent,
    mgr: &'static ManagerHandle,
}
AfbEventRegister!(SlacEvtCtrl, slac_event_cb, SlacEvtCtx);
fn slac_event_cb(evt: &AfbEventMsg, args: &AfbData, ctx: &mut SlacEvtCtx) -> Result<(), AfbError> {
    let msg = args.get::<&SlacStatus>(0)?;

    // forward slac events to potential listeners
    afb_log_msg!(Debug, evt, "slac_evt:{:?}", msg);
    ctx.evt.push(msg.clone());
    ctx.mgr.slac(evt, msg)?;

    Ok(())
}

struct IecEvtCtx {
    mgr: &'static ManagerHandle,
    evt: &'static AfbEvent,
}
AfbEventRegister!(IecEvtCtrl, iec_event_cb, IecEvtCtx);
fn iec_event_cb(evt: &AfbEventMsg, args: &AfbData, ctx: &mut IecEvtCtx) -> Result<(), AfbError> {
    let msg = args.get::<&Iec6185Msg>(0)?;

    afb_log_msg!(Debug, evt, "iec_evt:{:?}", msg.clone());
    ctx.evt.push(msg.clone());
    ctx.mgr.iec(evt, msg)?;

    Ok(())
}

struct SubscribeData {
    event: &'static AfbEvent,
}
AfbVerbRegister!(SubscribeCtrl, subscribe_callback, SubscribeData);
fn subscribe_callback(
    request: &AfbRequest,
    args: &AfbData,
    ctx: &mut SubscribeData,
) -> Result<(), AfbError> {
    let subcription = args.get::<bool>(0)?;
    if subcription {
        ctx.event.subscribe(request)?;
    } else {
        ctx.event.unsubscribe(request)?;
    }
    request.reply(AFB_NO_DATA, 0);
    Ok(())
}

pub(crate) fn register_verbs(api: &mut AfbApi, config: BindingCfg) -> Result<(), AfbError> {
    let event = AfbEvent::new("evt");
    let subscribe = AfbVerb::new("subscribe")
        .set_callback(Box::new(SubscribeCtrl { event }))
        .set_info("subscribe Iec6185 event")
        .set_usage("true|false")
        .finalize()?;
        // create charging manger handle
    let manager = ManagerHandle::new(config.auth_api, config.iec_api, config.engy_api);

    let iec_handler = AfbEvtHandler::new("iec-evt")
        .set_pattern(to_static_str(format!("{}/*", config.iec_api)))
        .set_callback(Box::new(IecEvtCtx {
            evt: event,
            mgr: manager,
        }))
        .finalize()?;

    let slac_handler = AfbEvtHandler::new("slac-evt")
        .set_pattern(to_static_str(format!("{}/*", config.slac_api)))
        .set_callback(Box::new(SlacEvtCtx { evt: event, mgr: manager }))
        .finalize()?;

    let engy_handler = AfbEvtHandler::new("engy-evt")
        .set_pattern(to_static_str(format!("{}/*", config.engy_api)))
        .set_callback(Box::new(EngyEvtCtx { evt: event, mgr: manager }))
        .finalize()?;

    api.add_evt_handler(engy_handler);
    api.add_evt_handler(iec_handler);
    api.add_evt_handler(slac_handler);
    api.add_event(event);
    api.add_verb(subscribe);

    Ok(())
}
