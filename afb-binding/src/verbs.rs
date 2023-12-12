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
use crate::prelude::*;
// use libchmgr::prelude::*;

struct IsoStateCtx {

}
AfbVerbRegister!(IsoStateVerb, iso_status_cb, IsoStateCtx);
fn iso_status_cb(rqt: &AfbRequest, args: &AfbData, _ctx: &mut IsoStateCtx) -> Result<(), AfbError> {
    match args.get::<&IsoState>(0)? {
       IsoState::PlugLock => { println!("plug+lock");},
       IsoState::PlugErr => { println!("plug+err");},
       IsoState::PlugIdle => { println!("unplugged");},
    }

    rqt.reply(AFB_NO_DATA, 0);
    Ok(())
}

pub(crate) fn register_verbs(api: &mut AfbApi, _config: BindingCfg) -> Result<(), AfbError> {

    let isostate = AfbVerb::new("isostate")
        .set_callback(Box::new(IsoStateCtx {

        }))
        .set_info("Update ISO/EIC states")
        .set_action("['PLUG-LOCK','PLUG-ERR','PLUG-IDLE']")?
        .finalize()?;

    api.add_verb(isostate);

    Ok(())
}
