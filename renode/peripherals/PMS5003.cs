//
// Copyright (c) 2023 lamb.jon.io@gmail.com
//
// This file is licensed under the MIT License.
// Full license text is available in 'licenses/MIT.txt'.
//
using Antmicro.Renode.Core;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.Bus;
using System.Collections.Generic;
using Antmicro.Renode.Core.Structure;
using System;
using Antmicro.Renode.Exceptions;
using Antmicro.Renode.Utilities;

namespace Antmicro.Renode.Peripherals.UART
{
    public class PMS5003 : UARTBase
    {
        public PMS5003(IMachine machine) : base(machine)
        {
        }

        protected override void CharWritten()
        {
            //this.NoisyLog("Char written. Count is {0}.", Count);
            if(Count == 7)
            {
                HandleCommand();
            }
        }

        protected override void QueueEmptied()
        {
            //this.NoisyLog("Queue empty.");
        }

        // TODO 
        // - clean this up
        // - add checksum handling
        // - add state for awake/asleep
        private void HandleCommand()
        {
            this.NoisyLog("Handling command");
            if(Count < 7)
            {
                return;
            }
            var buf = new byte[7];
            for (int i = 0; i < 7; i = i + 1) 
            {
                if(!TryGetCharacter(out var character))
                {
                    return;
                }
                buf[i] = (byte) character;
            }

            this.NoisyLog("Received : [{0:X}], Count={1}", BitConverter.ToString(buf), Count);

            if(buf[0] != 0x42 || buf[1] != 0x4D)
            {
                this.WarningLog("Bad header bytes");
                return;
            }

            var cmd = buf[2];
            switch(cmd)
            {
                case 0xE4: // wake/sleep
                    if(buf[4] == 0)
                    {
                        SendSleepResponse();
                    }
                    else
                    {
                        if(!is_init)
                        {
                            // TODO - see notes in fw
                            is_init = true;
                            SendOutput();
                        }
                    }
                    return;
                case 0xE1: // passive/active mode
                    if(buf[4] == 0)
                    {
                        SendPasiveModeResponse();
                    }
                    else
                    {
                        SendActiveModeResponse();
                    }
                    return;
                case 0xE2: // request data in passive mode
                    SendOutput();
                    return;
                default:
                    this.WarningLog("Unknown command {0:X}", cmd);
                    return;
            }
        }

        private void SendSleepResponse()
        {
            var data = new byte[] { 0x42, 0x4D, 0x00, 0x04, 0xE4, 0x00, 0x01, 0x77 };
            this.NoisyLog("Sending sleep cmd respose");
            SendResponse(data);
        }

        private void SendPasiveModeResponse()
        {
            var data = new byte[] { 0x42, 0x4D, 0x00, 0x04, 0xE1, 0x00, 0x01, 0x74 };
            this.NoisyLog("Sending passive mode respose");
            SendResponse(data);
        }

        private void SendActiveModeResponse()
        {
            var data = new byte[] { 0x42, 0x4D, 0x00, 0x04, 0xE1, 0x01, 0x01, 0x75 };
            this.NoisyLog("Sending active mode respose");
            SendResponse(data);
        }

        // TODO - add some wrappers to this, big endian
        private void SendOutput()
        {
            var data = new byte[32];
            data[0] = 0x42;
            data[1] = 0x4D;
            data[2] = 0; // frame_len
            data[3] = 32;
            data[12] = (byte) (pm2_5_atm >> 8);
            data[13] = (byte) (pm2_5_atm & 0xFF);
            var sum = crc(data);
            data[30] = (byte) (sum >> 8);
            data[31] = (byte) (sum & 0xFF);

            this.NoisyLog("Sending output data");
            SendResponse(data);
        }
        
        private void SendResponse(byte[] data)
        {
            foreach(var b in data)
            {
                TransmitCharacter(b);
            }
        }

        private uint crc(byte[] data)
        {
            var crc = (uint) 0;
            for(var i = 0; i < data.Length; i++)
            {
                crc += (uint) data[i];
            }
            return (uint) (crc & 0xFFFF);
        }

        public override Parity ParityBit
        {
            get
            {
                return Parity.None;
            }
        }

        public override Bits StopBits
        {
            get 
            { 
                return Bits.One;
            }
        }

        public override uint BaudRate
        {
            get
            {
                return 0;
            }
        }

        public uint PM2_5_atm
        {
            get => pm2_5_atm;
            set => pm2_5_atm = value.Clamp((uint) 0, (uint) 0xFFFF);
        }

        private uint pm2_5_atm = 0;
        private bool is_init = false;
    }
}

