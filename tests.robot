# tests.robot

# TODO
# - add a basic network ping test

*** Settings ***
Library  Process

# These are the defaults provided by renode, automatically set if not supplied
Suite Setup     Setup
Suite Teardown  Teardown
Test Teardown   Test Teardown
Resource        ${RENODEKEYWORDS}

*** Variables ***
# cargo test ... doesn't provide an easy way to get the binary filename
${GET_TEST_BIN_CMD}         cargo +nightly test --release --no-run --message-format=json | jq -r "select(.profile.test == true) | .filenames[]"
${PRODUCTION_BIN}           ${CURDIR}/target/thumbv7em-none-eabihf/release/air-gradient-pro
${BSP_PERIPHERALS}          SEPARATOR=\n
...                         """
...                         ledGreen: Miscellaneous.LED @ gpioPortB
...                         ledBlue: Miscellaneous.LED @ gpioPortB
...                         ledRed: Miscellaneous.LED @ gpioPortB
...
...                         gpioPortB:
...                         ${SPACE*4}0 -> ledGreen@0
...                         ${SPACE*4}7 -> ledBlue@0
...                         ${SPACE*4}14 -> ledRed@0
...
...                         phy: Network.EthernetPhysicalLayer @ ethernet 0
...                         ${SPACE*4}Id1: 0x0007
...                         ${SPACE*4}Id2: 0xC0F1
...                         ${SPACE*4}BasicStatus: 0xFE2D
...                         ${SPACE*4}AutoNegotiationAdvertisement: 0x00A1
...                         ${SPACE*4}AutoNegotiationLinkPartnerBasePageAbility: 0x001
...                         ${SPACE*4}VendorSpecific15: 0x101C
...                         """

${EXTERNAL_PERIPHERALS}     SEPARATOR=\n
...                         """
...                         ds3231 : Sensors.DS3231 @ i2c1 0x68
...
...                         sh1106 : Video.SH1106 @ i2c2 0x3C
...                         ${SPACE*4}FramesPerVirtualSecond: 10
...
...                         sgp41 : Sensors.SGP41 @ i2c2 0x59
...                         ${SPACE*4}VocTicks: 1024
...                         ${SPACE*4}NoxTicks: 2048
...
...                         sht31 : Sensors.SHT31 @ i2c2 0x44
...                         ${SPACE*4}Temperature: 40
...                         ${SPACE*4}Humidity: 85
...                         """

*** Keywords ***
Build Test Runner Firmware
    Run Process                 cargo +nighlty test --release --no-run  shell=true
    ${test_bin}=                Run Process     ${GET_TEST_BIN_CMD}     shell=true
    [return]                    ${test_bin.stdout}

Build Production Firmware
    Run Process                 cargo build --release   shell=true
    [return]                    ${PRODUCTION_BIN}

Start Firmware
    [Arguments]                 ${bin_path}
    Execute Command             mach create
    Execute Command             machine LoadPlatformDescription @platforms/cpus/stm32f429.repl
    Execute Command             machine LoadPlatformDescriptionFromString ${BSP_PERIPHERALS}
    Execute Command             machine LoadPlatformDescriptionFromString ${EXTERNAL_PERIPHERALS}
    Execute Command             sysbus LoadELF @${bin_path}
    Execute Command             emulation CreateSwitch "switch"
    Execute Command             connector Connect sysbus.ethernet switch
    Execute Command             logLevel 3
    Execute Command             sysbus.cpu PerformanceInMips 1
    Create Terminal Tester      sysbus.usart3
    Start Emulation

*** Test Cases ***
Execute Unit Tests
    [Documentation]             Runs the unit tests
    [Tags]                      unit_test  foo  bar

    ${bin}=                     Build Test Runner Firmware
    Start Firmware              ${bin}

    Wait For Line On Uart       running \\d+ tests              timeout=2   treatAsRegex=true
    Wait For Line On Uart       test result: ok.                timeout=10

Production Firmware Smoke Test
    [Documentation]             Runs the firmware, checks for basic things
    [Tags]                      production  foo

    ${bin}=                     Build Production Firmware
    Start Firmware              ${bin}

    Wait For Line On Uart       [I] Initialized                 timeout=10

Should Echo Back Over Network Using Tap
    # NOTE: this requires running the renode/setup-network.sh script as root for the tap
    [Documentation]             Firmware should echo back on port 12345
    [Tags]                      production  ethernet  tap

    Set Test Variable           ${TAP_INTERFACE}    renode-tap0
    Set Test Variable           ${SERVER_IP}        192.0.2.29
    Set Test Variable           ${SERVER_PORT}      12345
    Set Test Variable           ${CLIENT_REQ_CMD}   echo "hi" | socat -t 2 - udp:${SERVER_IP}:${SERVER_PORT}

    ${bin}=                     Build Production Firmware
    Start Firmware              ${bin}

    Execute Command             emulation CreateTap "${TAP_INTERFACE}" "tap"
    Execute Command             connector Connect host.tap switch

    # NOTE: workaround for https://github.com/renode/renode/issues/237
    Execute Command             sleep 3
    Execute Command             allowPrivates true
    Execute Command             sysbus.ethernet packetSent true
    Execute Command             sysbus.ethernet MAC "02:00:05:06:07:08"
    Execute Command             allowPrivates false

    Wait For Line On Uart       [I] Binding to UDP port 12345   timeout=10
    Wait For Line On Uart       [I] link=true                   timeout=5
    ${reply}=                   Run Process  ${CLIENT_REQ_CMD}  shell=true
    Should Contain              ${reply.stdout}  hello
