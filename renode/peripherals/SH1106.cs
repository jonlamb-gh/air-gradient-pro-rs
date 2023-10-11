//
// Copyright (c) 2023 Jon Lamb (lamb.jon.io@gmail.com)
// Copyright (c) 2010-2020 Antmicro
//
//  This file is licensed under the MIT License.
//  Full license text is available in 'licenses/MIT.txt'.
//
using System;
using System.Linq;
using System.Collections;
using Antmicro.Renode.Core;
using Antmicro.Renode.Core.Structure;
using Antmicro.Renode.Core.Structure.Registers;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Utilities;
using Antmicro.Renode.Backends.Display;
using Antmicro.Renode.Peripherals.I2C;
// TODO
// maybe based on this
// https://github.com/renode/renode-infrastructure/blob/master/src/Emulator/Peripherals/Peripherals/Video/PL110.cs
//
// https://github.com/renode/renode-infrastructure/blob/master/src/Emulator/Peripherals/Peripherals/I2C/LC709205F.cs
//
// https://www.velleman.eu/downloads/29/infosheets/sh1106_datasheet.pdf
namespace Antmicro.Renode.Peripherals.Video
{
    public class SH1106 : AutoRepaintingVideo, II2CPeripheral, IProvidesRegisterCollection<ByteRegisterCollection>
    {
        public SH1106(Machine machine) : base(machine)
        {
            // TODO - not sure using RegistersCollection makes sense here
            //
            // Using RegistersCollection isn't necessary, it's just 0x00 commands and 0x40 data bytes
            // just do like the PL110 impl does
            // this.LogUnhandledWrite(...);
            RegistersCollection = new ByteRegisterCollection(this);
            DefineRegisters();

            // Using PixelFormat.A8 to represent monochrome
            Reconfigure(DisplayWidth, DisplayHeight, PixelFormat.A8);
            this.machine = machine;

            Reset();
        }

        public override void Reset()
        {
            RegistersCollection.Reset();
            registerAddress = null;

            // TODO
            displayOn = false;
            allOn = false;
            invert = false;
        }

        public void Write(byte[] data)
        {
            if(data.Length == 0)
            {
                this.Log(LogLevel.Warning, "Unexpected write with no data");
                return;
            }

            registerAddress = (Registers) data[0];
            this.Log(LogLevel.Noisy, "Writing {0} bytes to register {1} (0x{1:X})", data.Length, registerAddress);

            if((registerAddress == Registers.Command) && (data.Length >= 2))
            {
                WriteCommand(data);
            }
            else if((registerAddress == Registers.Data) && (data.Length >= 2))
            {
                WriteData(data);
            }
            else
            {
                // TODO - once migrated away from IProvidesRegisterCollection
                // invalid length
                this.Log(LogLevel.Error, "Invalid length");
                //this.LogUnhandledWrite(data[0]);
            }
        }

        private void WriteCommand(byte[] data)
        {
            this.Log(LogLevel.Noisy, "Command: {0:X}", BitConverter.ToString(data));

            var cmd = data[1];
            var cmdHandled = true;

            // Includes command byte 0x00 at [0]
            if(data.Length == 2)
            {
                if(displayOn && ((cmd & 0xF0) == 0xB0)) // PageAddress
                {
                    drawRow = data[1] & (uint) 0x0F;
                }
                else if(displayOn && ((cmd & 0xF0) == 0x00)) // ColumnAddressLow
                {
                    drawColumn = data[1] & (uint) 0x0F;
                }
                else if(displayOn && ((cmd & 0xF0) == 0x10)) // ColumnAddressHigh
                {
                    drawColumn |= ((data[1] & (uint) 0x0F) << 4);
                }
                else if((cmd & 0xFE) == 0xAE) // DisplayOn
                {
                    displayOn = (cmd & 0x01) != 0;
                }
                else if((cmd & 0xFE) == 0xA4) // AllOn
                {
                    // TODO - this should set all pixels to on, otherwise use what's in mem
                    allOn = (cmd & 0x01) != 0;
                }
                else if((cmd & 0xFE) == 0xA6) // Invert
                {
                    invert = (cmd & 0x01) != 0;
                }
                else if((cmd & 0xA0) == 0xA0) // SegmentRemap
                {
                    // no-op
                }
                else if((cmd & 0xC0) == 0xC0) // ReverseComDir
                {
                    // no-op
                }
                else if((cmd & 0x40) == 0x40) // StartLine
                {
                    var startLine = cmd & 0x3F;
                    if(startLine != 0)
                    {
                        this.Log(LogLevel.Warning, "Unsupported StartLine {0}", startLine);
                    }
                }
                else
                {
                    cmdHandled = false;
                }
            }
            else if(data.Length == 3)
            {
                if(cmd == 0x81) // Contrast
                {
                    // no-op
                }
                else if(cmd == 0xA8) // Multiplex
                {
                    // no-op
                    var mux = data[2];
                    if(mux != DisplayHeight - 1)
                    {
                        this.Log(LogLevel.Warning, "Unsupported Multiplex {0}", mux);
                    }
                }
                else if(cmd == 0xD3) // DisplayOffset
                {
                    var offset = data[2];
                    if(offset != 0)
                    {
                        this.Log(LogLevel.Warning, "Unsupported offset {0}", offset);
                    }
                }
                else if(cmd == 0xDA) // ComPinConfig
                {
                    // no-op
                }
                else if(cmd == 0xD5) // DisplayClockDiv
                {
                    // no-op
                }
                else if(cmd == 0xD9) // PreChargePeriod
                {
                    // no-op
                }
                else if(cmd == 0xDB) // VcomhDeselect
                {
                    // no-op
                }
                else if(cmd == 0xAD) // ChargePump
                {
                    // no-op
                    // TODO Display must be off when performing this command
                }
                else
                {
                    cmdHandled = false;
                }
            }
            else if((data.Length == 4) && (cmd == 0x00))
            {
                pageAddress = data[1];
                if(data[2] != 0x02)
                {
                    this.Log(LogLevel.Warning, "Unsupported lower column address 0x{0:X}", data[2]);
                }
                if(data[3] != 0x10)
                {
                    this.Log(LogLevel.Warning, "Unsupported upper column address 0x{0:X}", data[3]);
                }
            }
            else
            {
                cmdHandled = false;
            }

            if(!cmdHandled)
            {
                this.Log(LogLevel.Warning, "Unhandled write to command 0x{0:X}", cmd);
            }
        }

        private void WriteData(byte[] data)
        {
            // Includes command byte 0x40 at [0]

            if(pageAddress == null)
            {
                this.Log(LogLevel.Warning, "Missing page address");
                return;
            }

            pageAddress = null;

            if(!displayOn)
            {
                this.Log(LogLevel.Warning, "Display is not on");
                return;
            }

            if(data.Length != (DisplayWidth + 1))
            {
                this.Log(LogLevel.Warning, "Invalid pixel data length");
                return;
            }

            this.Log(LogLevel.Noisy, "Data pageAddress=0x{0:X}, drawRow={1}, drawColumn={2}",
                    pageAddress, drawRow, drawColumn);

            for(var byte_idx = 0; byte_idx < data.Length - 1; byte_idx++)
            {
                // First byte is command 0x40
                var byte_in = data[1 + byte_idx];
                var col_x = byte_idx;

                BitArray bits = new BitArray(new byte[] {byte_in});
                for(var i = 0; i < bits.Length; i++)
                {
                    var row_y = (drawRow * 8) + i;
                    var buf_idx = (row_y * DisplayWidth) + col_x;

                    // TODO invert support
                    buffer[buf_idx] = (byte) (bits[i] == true ? 0xFF : 0);
                }
            }
        }

        public byte[] Read(int count)
        {
            // TODO
            //this.LogUnhandledRead(registerAddress);
            this.Log(LogLevel.Warning, "Reading is not supported");
            return new byte[] {};
        }

        public void FinishTransmission()
        {
            registerAddress = null;
        }

        public ByteRegisterCollection RegistersCollection { get; }

        protected override void Repaint()
        {
            // Nothing to do here
        }

        private void DefineRegisters()
        {
            Registers.Command.Define(this)
                .WithValueField(0, 8, FieldMode.Write, name: "CMD")
            ;
            Registers.Data.Define(this)
                .WithValueField(0, 8, FieldMode.Write, name: "DATA")
            ;
        }

        private Registers? registerAddress;

        private bool displayOn;
        private bool allOn;
        private bool invert;
        private uint drawRow;
        private uint drawColumn;
        private uint? pageAddress;

        // TODO
        private readonly Machine machine;
        private const int DisplayWidth = 128;
        private const int DisplayHeight = 64;

        private enum Registers : byte
        {
            Command = 0x00,
            Data = 0x40,
        }
    }
}
