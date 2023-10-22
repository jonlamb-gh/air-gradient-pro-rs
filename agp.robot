*** Settings ***
Documentation   Integration tests for air-gradient-pro bootloader, firmware and CLI
Default Tags    agp
Library         Process

# These are the defaults provided by renode, automatically set if not supplied
Suite Setup     AGP Suite Setup
Suite Teardown  Teardown
Test Setup      AGP Test Setup
Test Teardown   Test Teardown
Resource        ${RENODEKEYWORDS}

*** Variables ***
# Firmware configs
${AIR_GRADIENT_MAC_ADDRESS}     02:00:04:03:07:04
${AIR_GRADIENT_IP_ADDRESS}      192.0.2.80
${AIR_GRADIENT_DEVICE_ID}       255
${AIR_GRADIENT_DEVICE_PORT}     32101
${AIR_GRADIENT_LOG}             TRACE
# Test vars
${CLI}                          ${CURDIR}/host_tools/air-gradient-cli/target/x86_64-unknown-linux-gnu/release/air-gradient
${AGP_RESC}                     ${CURDIR}/renode/agp.resc
${FW_IMAGE}                     ${CURDIR}/firmware/target/agp_images.cpio
${UART}                         sysbus.usart6
${UART_TIMEOUT}                 30
${PING_TIMEOUT}                 5
${VERBOSE_LOGGING_DIR}          ${CURDIR}/agp_logs
${VERBOSE_LOGGING_ENABLED}      True
${RENODE_LOG_LEVEL}             2

*** Keywords ***
AGP Suite Setup
    Setup
    Build System

AGP Test Setup
    ${test_name} =                  Replace String      ${TEST NAME}        ${SPACE}  _
    Set Test Variable               \${TEST_NAME}       ${test_name}
    Test Setup
    Prepare Machine

Build Firmware
    Set environment variable        AIR_GRADIENT_MAC_ADDRESS    ${AIR_GRADIENT_MAC_ADDRESS}
    Set environment variable        AIR_GRADIENT_IP_ADDRESS     ${AIR_GRADIENT_IP_ADDRESS}
    Set environment variable        AIR_GRADIENT_DEVICE_ID      ${AIR_GRADIENT_DEVICE_ID}
    Set environment variable        AIR_GRADIENT_LOG            ${AIR_GRADIENT_LOG}
    Set environment variable        AIR_GRADIENT_DEVICE_PORT    ${AIR_GRADIENT_DEVICE_PORT}
 
    ${result} =                     Run Process         cargo build --release       cwd=firmware  shell=true
    IF                              ${result.rc} != 0
        Log To Console              ${result.stdout}    console=yes
        Log To Console              ${result.stderr}    console=yes
    END
    Should Be Equal As Integers     ${result.rc}        0

Build Bootloader
    Set environment variable        AIR_GRADIENT_LOG    ${AIR_GRADIENT_LOG}
    ${result} =                     Run Process         cargo build --release       cwd=bootloader  shell=true
    IF                              ${result.rc} != 0
        Log To Console              ${result.stdout}    console=yes
        Log To Console              ${result.stderr}    console=yes
    END
    Should Be Equal As Integers     ${result.rc}        0

Build CLI
    ${result} =                     Run Process         cargo build --release       cwd=host_tools/air-gradient-cli  shell=true
    IF                              ${result.rc} != 0
        Log To Console              ${result.stdout}    console=yes
        Log To Console              ${result.stderr}    console=yes
    END
    Should Be Equal As Integers     ${result.rc}        0

Build System
    Build Firmware
    Build Bootloader
    Build CLI

Enable Verbose Logging
    Create Directory                ${VERBOSE_LOGGING_DIR}

    Execute Command                 usart6 CreateFileBackend @${VERBOSE_LOGGING_DIR}/${TEST_NAME}_uart.txt true
    Execute Command                 logFile @${VERBOSE_LOGGING_DIR}/${TEST_NAME}_renode.log true
    Execute Command                 logLevel ${RENODE_LOG_LEVEL} file

Prepare Machine
    Execute Command                 path add @${CURDIR}
    Execute Script                  ${AGP_RESC}
    Create Terminal Tester          ${UART}             timeout=${UART_TIMEOUT}
    IF                              ${VERBOSE_LOGGING_ENABLED} == True
        Enable Verbose Logging
    END

Run Command
    [Arguments]                     ${cmd_and_args}
    IF                              ${VERBOSE_LOGGING_ENABLED} == True
        ${result} =                 Run Process         ${cmd_and_args}  shell=true  stdout=${VERBOSE_LOGGING_DIR}/${TEST_NAME}_cmd.stdout  stderr=${VERBOSE_LOGGING_DIR}/${TEST_NAME}_cmd.stderr
    ELSE
        ${result} =                 Run Process         ${cmd_and_args}  shell=true
    END

    IF                              ${result.rc} != 0
        Log To Console              ${result.stdout}    console=yes
        Log To Console              ${result.stderr}    console=yes
    END

    Should Be Equal As Integers     ${result.rc}        0

    RETURN                          ${result}

Run CLI Command
    [Arguments]                     ${args}             ${log_level}=error
    ${result} =                     Run Command         RUST_LOG=${log_level} ${CLI} ${args}
    RETURN                          ${result}

Wait For Device Networking
    Wait For Line On Uart           [D] UM: listening on port ${AIR_GRADIENT_DEVICE_PORT}

*** Test Cases ***
System Boots
    [Documentation]                 Boots the system, bootloader and firmware
    [Tags]                          firmware  bootloader  uart

    Start Emulation

    Wait For Line On Uart           [W] Invalid boot config, using default
    Wait For Line On Uart           agp-bootloader
    Wait For Line On Uart           Reset reason: Power-on reset
    Wait For Line On Uart           Boot config slot: SLOT0
    Wait For Line On Uart           Update pending: false
    Wait For Line On Uart           Update valid: false
    Wait For Line On Uart           air-gradient-pro-rs
    Wait For Line On Uart           Reset reason: Power-on reset
    Wait For Line On Uart           Update pending: false
    Wait For Line On Uart           >>> Initialized <<<

Responds to Ping
    [Documentation]                 TCP/IP stack should respond to pings
    [Tags]                          firmware  bootloader  uart  network

    Start Emulation

    Set Test Variable               ${PING_CMD}         ping -w ${PING_TIMEOUT} -W 1 ${AIR_GRADIENT_IP_ADDRESS}

    Wait For Device Networking

    ${result} =                     Run Process         ${PING_CMD}  shell=true
    Should Be Equal As Integers     ${result.rc}        0

Responds with Device Info
    [Documentation]                 Device responds to info request
    [Tags]                          firmware  bootloader  network  device-protocol  cli

    Start Emulation

    Wait For Device Networking

    ${result} =                     Run CLI Command     device info -a ${AIR_GRADIENT_IP_ADDRESS} -F device_id
    Should Be Equal As Integers     ${result.stdout}    ${AIR_GRADIENT_DEVICE_ID}

    Wait For Line On Uart           [D] UM: processing command Info

# NOTE: this test flaps sometimes on my local machine but not in CI...
Device Applies Valid Firmware Update
    [Documentation]                 Devices handles firmware updates from the CLI, initially boots from SLOT0,
    ...                             updates and boots to SLOT1
    [Tags]                          firmware  bootloader  network  device-protocol  cli  fota

    Start Emulation

    Wait For Device Networking

    ${r0} =                         Run CLI Command      device info -a ${AIR_GRADIENT_IP_ADDRESS} -F active_boot_slot
    Should Be Equal                 ${r0.stdout}        "SLOT0"

    # Socket gets reset every time the connection ends
    Wait For Device Networking

    Run CLI Command                 device update -a ${AIR_GRADIENT_IP_ADDRESS} ${FW_IMAGE}  log_level=trace

    # firmware msg when upload complete
    Wait For Line On Uart           [W] Update complete, rebooting now

    # bootloader msg after reboot
    Wait For Line On Uart           Update pending: true

    # firmware ACKs the pending update and resets
    Wait For Line On Uart           New application update checks out, marking for BC flash and reseting

    # bootloader msg after next reboot from ACK
    Wait For Line On Uart           Update valid: true

    Wait For Device Networking

    ${r1} =                         Run CLI Command     device info -a ${AIR_GRADIENT_IP_ADDRESS} -F active_boot_slot
    Should Be Equal                 ${r1.stdout}        "SLOT1"

# TODO
# - tests for fota related, failover mechanism, etc
# - device protocol and CLI ops
# - sensors and bcast protocol stuff
# - watchdog and panic stuf
