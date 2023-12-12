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
use serde::{Deserialize, Serialize};

AfbDataConverter!(error_state, ErrorState);
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE", untagged)]
pub enum ErrorState {
    ErrE,
    ErrDf,
    ErrRelay,
    ErrRdc,
    ErrOverCurrent,
    ErrPermanent,
    ErrVentilation,
}

AfbDataConverter!(iec_state, IecState);
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE", tag = "action")]
pub enum IecState {
    Bdf,
    Ef,
    Unset,
}

AfbDataConverter!(power_request, PowerRequest);
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE", tag = "action")]
pub enum PowerRequest {
    Start,
    Stop,
}

AfbDataConverter!(plug_state, PlugState);
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE", tag = "action")]
pub enum PlugState {
    PlugIn,
    Lock,
    Error,
    PlugOut,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE", tag = "action")]
pub enum Iso15118State {
    Iso20,
    Iso2,
    Iec,
    Unset,
}

AfbDataConverter!(vehicle_state, VehicleState);
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case", tag = "action")]
pub struct VehicleState {
    pub plugged: PlugState,
    pub power_request: PowerRequest,
    pub power_imax: u32,
    pub iso15118: Iso15118State,
    pub iec_state: IecState,
}

pub fn types_register() -> Result<(),AfbError> {
    iec_state::register()?;
    plug_state::register()?;
    vehicle_state::register()?;
    error_state::register()?;
    power_request::register()?;

    Ok(())
}