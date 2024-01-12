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

AfbDataConverter!(auth_state, AuthState);
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE", untagged)]
pub enum AuthState {
    Done,
    Fail,
    Pending,
    Idle,
}


pub fn auth_registers() -> Result<(),AfbError> {
    auth_state::register()?;

    Ok(())
}