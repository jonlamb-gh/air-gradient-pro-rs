#!/usr/bin/env bash

BRIDGE="demo-network"
HWADDR="00:00:5e:01:23:ff"
ADDR="192.0.2.2/24"
ROUTE="192.0.2.0/24"
RENODETAP="renode-tap0"

STOPPED=0
trap ctrl_c INT TERM

ctrl_c() {
    STOPPED=1
}

echo "Adding bridge '$BRIDGE'"

brctl addbr $BRIDGE
ip address add $ADDR dev $BRIDGE
ip link set dev $BRIDGE up
ip link set dev $BRIDGE address $HWADDR
ip route add $ROUTE dev $BRIDGE > /dev/null 2>&1

echo "Adding tap '$RENODETAP'"
ip tuntap add dev $RENODETAP mode tap
ip link set dev $RENODETAP up
brctl addif $BRIDGE $RENODETAP

while [ $STOPPED -eq 0 ]; do
    sleep 1d
done

ip tuntap del $RENODETAP mode tap

ip link set $BRIDGE down

echo "Deleting $BRIDGE"

brctl delbr $BRIDGE
