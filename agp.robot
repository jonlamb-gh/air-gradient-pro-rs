*** Settings ***
Documentation   Integration tests for air-gradient-pro bootloader, firmware and CLI
Default Tags    agp
Library         Process

*** Variables ***
# Firmware configs
${AIR_GRADIENT_MAC_ADDRESS}     02:00:04:03:07:04
${AIR_GRADIENT_IP_ADDRESS}      192.0.2.80
${AIR_GRADIENT_DEVICE_ID}       255
${AIR_GRADIENT_LOG}             TRACE
# Test vars
${AGP_RESC}                     ${CURDIR}/renode/agp.resc
${UART}                         sysbus.usart6
${UART_TIMEOUT}                 10

*** Keywords ***
Build Firmware
    Set environment variable    AIR_GRADIENT_MAC_ADDRESS    ${AIR_GRADIENT_MAC_ADDRESS}
    Set environment variable    AIR_GRADIENT_IP_ADDRESS     ${AIR_GRADIENT_IP_ADDRESS}
    Set environment variable    AIR_GRADIENT_DEVICE_ID      ${AIR_GRADIENT_DEVICE_ID}
    Set environment variable    AIR_GRADIENT_LOG            ${AIR_GRADIENT_LOG}
    ${result}=  Run Process     cargo build --release       cwd=firmware  shell=true
    #Log To Console              ${result.stdout} console=yes
    #Log To Console              ${result.stderr} console=yes
    Should Be Equal As Integers  ${result.rc}               0

Build Bootloader
    Set environment variable    AIR_GRADIENT_LOG            ${AIR_GRADIENT_LOG}
    ${result}=  Run Process     cargo build --release       cwd=bootloader  shell=true
    #Log To Console              ${result.stdout} console=yes
    #Log To Console              ${result.stderr} console=yes
    Should Be Equal As Integers  ${result.rc}               0

Prepare Machine
    Execute Command             path add @${CURDIR}
    Execute Script              ${AGP_RESC}

*** Test Cases ***
Boot the System
    [Documentation]             Boots the system, bootloader and firmware
    [Tags]                      firmware  bootloader  uart

    Build Firmware
    Build Bootloader

    Prepare Machine
    Create Terminal Tester      ${UART}  timeout=${UART_TIMEOUT}

    Start Emulation

    Wait For Line On Uart       >>> Initialized <<<

    Provides                    booted-system
