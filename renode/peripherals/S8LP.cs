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
    public class S8LP : UARTBase
    {
        public S8LP(IMachine machine) : base(machine)
        {
        }

        protected override void CharWritten()
        {
            //this.NoisyLog("Char written. Count is {0}.", Count);
            if(Count == 8)
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
        // - need modbus utils
        // - add checksum handling
        private void HandleCommand()
        {
            this.NoisyLog("Handling command");
            if(Count < 8)
            {
                return;
            }
            var buf = new byte[8];
            for (int i = 0; i < 8; i = i + 1) 
            {
                if(!TryGetCharacter(out var character))
                {
                    return;
                }
                buf[i] = (byte) character;
            }

            this.NoisyLog("Received : [{0:X}]", BitConverter.ToString(buf));

            if(buf[0] != 0xFE || buf[1] != 0x04)
            {
                this.WarningLog("Bad header bytes");
                return;
            }

            SendOutput();
        }

        // TODO - add some wrappers to this, big endian
        private void SendOutput()
        {
            var data = new byte[7 - 2];
            data[0] = 0xFE;
            data[1] = 0x04;
            data[2] = 2;
            data[3] = (byte) (co2 >> 8);
            data[4] = (byte) (co2 & 0xFF);

            this.NoisyLog("Sending output data");
            SendResponseAndCrc(data);
        }
        
        private void SendResponseAndCrc(byte[] data)
        {
            foreach(var b in data)
            {
                TransmitCharacter(b);
            }
            var sum = crc(data);
            TransmitCharacter((byte) (sum & 0xFF));
            TransmitCharacter((byte) (sum >> 8));
        }

        private uint crc(byte[] data)
        {
            var crc = (uint) 0xFFFF;
            for(var i = 0; i < data.Length; i++)
            {
                crc ^= (uint) data[i];
                for(var j = 0; j < 8; j++)
                {
                    var crc_odd = (crc & 0x0001) != 0;
                    crc = crc >> 1;
                    if(crc_odd)
                    {
                        crc ^= 0xA001;
                    }
                }
            }
            return crc;
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

        public uint Co2
        {
            get => co2;
            set => co2 = value.Clamp((uint) 0, (uint) 0xFFFF);
        }

        private uint co2 = 0;
    }
}

