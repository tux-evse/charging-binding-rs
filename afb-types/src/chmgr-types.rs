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
use serde::{Deserialize, Serialize};

AfbDataConverter!(error_state, ErrorState);
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ErrorState {
    ErrE,
    ErrDf,
    ErrRelay,
    ErrRdc,
    ErrOverCurrent,
    ErrPermanent,
    ErrVentilation,
}

AfbDataConverter!(power_request, PowerRequest);
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PowerRequest {
    Start,
    Charging(u32),
    Stop(i32),
    Idle,
}

AfbDataConverter!(plug_state, PlugState);
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PlugState {
    PlugIn,
    Lock,
    Error,
    PlugOut,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum IsoState {
    Iso20,
    Iso2,
    Iec,
    Unset,
}

AfbDataConverter!(charging_event, ChargingMsg);
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ChargingMsg {
    Plugged(PlugState),
    Power(PowerRequest),
    Iso(IsoState),
    Auth(AuthMsg),
    State(ChargingState),
}

AfbDataConverter!(charging_state, ChargingState);
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub struct ChargingState {
    #[serde(skip)]
    pub updated: bool,
    pub imax: u32,
    pub pmax: u32,
    pub plugged: PlugState,
    pub power: PowerRequest,
    pub iso: IsoState,
    pub auth: AuthMsg,
}

impl ChargingState {
    pub fn default() -> Self {
        ChargingState {
            updated: false,
            imax: 0,
            pmax:0,
            plugged: PlugState::Unknown,
            power: PowerRequest::Idle,
            iso: IsoState::Unset,
            auth: AuthMsg::Idle,
        }
    }
}

AfbDataConverter!(charging_actions, ChargingAction);
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "lowercase", tag = "action")]
pub enum ChargingAction {
    #[default]
    READ,
    SUBSCRIBE,
    UNSUBSCRIBE,
}

pub fn chmgr_registers() -> Result<(), AfbError> {
    charging_actions::register()?;
    plug_state::register()?;
    charging_state::register()?;
    error_state::register()?;
    power_request::register()?;
    charging_event::register()?;

    Ok(())
}
