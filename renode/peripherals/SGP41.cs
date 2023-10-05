//
// Copyright (c) 2023 Jon Lamb (lamb.jon.io@gmail.com)
// Copyright (c) 2010-2023 Antmicro
//
// This file is licensed under the MIT License.
// Full license text is available in 'licenses/MIT.txt'.
//
using System;
using System.Collections.Generic;
using System.Linq;
using Antmicro.Renode.Exceptions;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.I2C;
using Antmicro.Renode.Peripherals.Sensor;
using Antmicro.Renode.Utilities;

namespace Antmicro.Renode.Peripherals.Sensors
{
    public class SGP41 : II2CPeripheral
    {
        public SGP41()
        {
            commands = new I2CCommandManager<Action<byte[]>>();
            outputBuffer = new Queue<byte>();

            commands.RegisterCommand(SoftReset, 0x00, 0x06);
            commands.RegisterCommand(ExecuteConditioning, 0x26, 0x12);
            commands.RegisterCommand(MeasureRawSignals, 0x26, 0x19);
            commands.RegisterCommand(ExecuteSelfTest, 0x28, 0x0E);
            commands.RegisterCommand(TurnHeaterOff, 0x36, 0x15);
            commands.RegisterCommand(GetSerialNumber, 0x36, 0x82);

            Reset();
        }

        public byte[] Read(int count)
        {
            var result = outputBuffer.ToArray();
            this.Log(LogLevel.Noisy, "Reading {0} bytes from the device", result.Length);
            outputBuffer.Clear();
            return result;
        }

        // TODO - need to do CRC checks...
        public void Write(byte[] data)
        {
            this.Log(LogLevel.Noisy, "Received {0} bytes: [{1:X}]", data.Length, BitConverter.ToString(data));
            if(!commands.TryGetCommand(data, out var command))
            {
                this.Log(LogLevel.Warning, "Unknown command: [{0}]. Ignoring the data.", string.Join(", ", data.Select(x => string.Format("0x{0:X}", x))));
                return;
            }
            command(data);
        }

        public void FinishTransmission()
        {
            // Nothing to do
        }

        public ulong SerialNumber
        {
            get => serialNumber;
            set => serialNumber = value.Clamp((ulong) 0, (ulong) 0xFFFFFFFFFFFF);
        }

        public uint VocTicks
        {
            get => vocTicks;
            set => vocTicks = value.Clamp((uint) 0, (uint) 0xFFFF);
        }

        public uint NoxTicks
        {
            get => noxTicks;
            set => noxTicks = value.Clamp((uint) 0, (uint) 0xFFFF);
        }

        public void Reset()
        {
            // These are publicly configurable, keep user-provided values persistent
            //vocTicks = 0;
            //noxTicks = 0;
            outputBuffer.Clear();
        }

        private void SoftReset(byte[] command)
        {
            this.Log(LogLevel.Noisy, "Performing soft reset");
            Reset();
        }

        private void ExecuteConditioning(byte[] command)
        {
            this.Log(LogLevel.Noisy, "Executing conditioning");
            // TODO check command data and len
            var res = new byte[] {0, 0};
            Enqueue2AndCrc(res);
        }

        private void MeasureRawSignals(byte[] command)
        {
            this.Log(LogLevel.Noisy, "Measuring raw signals");

            // TODO check command data and len
            // with and without humidity compensation
            // without has fixed values

            var buf = new byte[] {(byte) (vocTicks >> 8), (byte) (vocTicks & 0xFF)};
            Enqueue2AndCrc(buf);

            buf[0] = (byte) (noxTicks >> 8);
            buf[1] = (byte) (noxTicks & 0xFF);
            Enqueue2AndCrc(buf);
        }

        private void ExecuteSelfTest(byte[] command)
        {
            // TODO
            this.Log(LogLevel.Noisy, "Executing self test");
            // TODO check command data and len
            var res = new byte[] {0, 0};
            Enqueue2AndCrc(res);
        }

        private void TurnHeaterOff(byte[] command)
        {
            this.Log(LogLevel.Noisy, "Turning off heater - entering idle mode");
        }

        private void GetSerialNumber(byte[] command)
        {
            this.Log(LogLevel.Noisy, "Reading serial number");
            var buf = new byte[] {(byte) (serialNumber >> 40), (byte) (serialNumber >> 32)};
            Enqueue2AndCrc(buf);
            buf[0] = (byte) (serialNumber >> 24);
            buf[1] = (byte) (serialNumber >> 16);
            Enqueue2AndCrc(buf);
            buf[0] = (byte) (serialNumber >> 8);
            buf[1] = (byte) (serialNumber & 0xFF);
            Enqueue2AndCrc(buf);
        }

        private void Enqueue2AndCrc(byte[] data)
        {
            foreach(var b in data)
            {
                outputBuffer.Enqueue(b);
            }
            outputBuffer.Enqueue(crc(data));
        }

        private byte crc(byte[] data)
        {
            var crc = (byte) 0xFF;
            for(var i = 0; i < 2; i++)
            {
                crc ^= data[i];
                for(var bit = 8; bit > 0; --bit)
                {
                    if((crc & 0x80) != 0)
                    {
                        crc = (byte) ((crc << 1) ^ 0x31);
                    }
                    else
                    {
                        crc = (byte) (crc << 1);
                    }
                }
            }
            return crc;
        }

        // NOTE: all fields are sent big-endian
        private ulong serialNumber = 0xAABBCCDDEEFF;
        private uint vocTicks = 0;
        private uint noxTicks = 0;

        private readonly I2CCommandManager<Action<byte[]>> commands;
        private readonly Queue<byte> outputBuffer;
    }
}
