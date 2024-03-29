#!/bin/bash

export LD_LIBRARY_PATH=/usr/local/lib64
pkill afb-chmgr
cynagora-admin set '' 'HELLO' '' '*' yes
clear

# build test config dirname
DIRNAME=`dirname $0`
cd $DIRNAME/..
CONFDIR=`pwd`/etc
mkdir -p /tmp/api

DEVTOOL_PORT=1237
echo Slac debug mode config=$CONFDIR/*.json port=$DEVTOOL_PORT

afb-binder --name=afb-chmgr --port=$DEVTOOL_PORT -v \
  --config=$CONFDIR/binder-chmgr.json \
  --config=$CONFDIR/binding-chmgr.json \
  --config=$CONFDIR/binding-slac.json \
  --config=$CONFDIR/binding-am62x.json \
  --config=$CONFDIR/binding-i2c.json \
  $*