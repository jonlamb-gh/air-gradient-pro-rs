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
    public class SHT31 : II2CPeripheral, ITemperatureSensor, IHumiditySensor
    {
        public SHT31()
        {
            commands = new I2CCommandManager<Action<byte[]>>();
            outputBuffer = new Queue<byte>();

            commands.RegisterCommand(SingleShotMeasurement, 0x24); // 0x24 0xXX
            commands.RegisterCommand(FetchData, 0xE0, 0x00);
            commands.RegisterCommand(SoftReset, 0x30, 0xA2);
            commands.RegisterCommand(ClearStatus, 0x30, 0x41);
            commands.RegisterCommand(Status, 0xF3, 0x2D);
            commands.RegisterCommand(ReadSerialNumber, 0x37, 0x80);

            Reset();
        }

        public byte[] Read(int count)
        {
            var result = outputBuffer.ToArray();
            this.Log(LogLevel.Noisy, "Reading {0} bytes from the device", result.Length);
            outputBuffer.Clear();
            return result;
        }

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

        public void Reset()
        {
            // These are publicly configurable, keep user-provided values persistent
            //Temperature = 0;
            //Humidity = 0;
            outputBuffer.Clear();
        }

        // TODO
        public decimal Humidity
        {
            get
            {
                var rh = 100 * (((decimal) humidity) / 65535);
                this.Log(LogLevel.Noisy, "Getting humidity={0} raw={1}", rh, humidity);
                return rh;
            }
            set
            {
                if(MinHumidity > value || value > MaxHumidity)
                {
                    throw new RecoverableException("The humidity value must be between {0} and {1}.".FormatWith(MinHumidity, MaxHumidity));
                }
                humidity = (uint) ((value / 100) * 65536);
                this.Log(LogLevel.Noisy, "Setting humidity={0} raw={1}", value, humidity);
            }
        }

        public decimal Temperature
        {
            get
            {
                var temp_c = -45 + (175 * (((decimal) temperature) / 65535));
                this.Log(LogLevel.Noisy, "Getting temperature={0} raw={1}", temp_c, temperature);
                return temp_c;
            }
            set
            {
                if(MinTemperature > value || value > MaxTemperature)
                {
                    throw new RecoverableException("The temperature value must be between {0} and {1}.".FormatWith(MinTemperature, MaxTemperature));
                }
                temperature = (uint) (65535 * ((value + 45) / 175));
                this.Log(LogLevel.Noisy, "Setting temperature={0} raw={1}", value, temperature);
            }
        }

        public uint SerialNumber
        {
            get => serialNumber;
            set => serialNumber = value.Clamp((uint) 0, (uint) 0xFFFF);
        }

        // TODO - add all the variants
        // Repeatability
        // High: 0x00
        // Medium: 0x0B
        // Low: 0x016
        private void SingleShotMeasurement(byte[] command)
        {
            this.Log(LogLevel.Noisy, "Measuring single shot mode ClockStretch=false Repeatability=high");

            // TODO check command data and len

            var buf = new byte[2];
            buf[0] = (byte) (temperature >> 8);
            buf[1] = (byte) (temperature & 0xFF);
            Enqueue2AndCrc(buf);
            buf[0] = (byte) (humidity >> 8);
            buf[1] = (byte) (humidity & 0xFF);
            Enqueue2AndCrc(buf);
        }
        
        private void FetchData(byte[] command)
        {
            this.Log(LogLevel.Noisy, "Fetch data");

            // TODO check command data and len

            var buf = new byte[2];
            buf[0] = (byte) (temperature >> 8);
            buf[1] = (byte) (temperature & 0xFF);
            Enqueue2AndCrc(buf);
            buf[0] = (byte) (humidity >> 8);
            buf[1] = (byte) (humidity & 0xFF);
            Enqueue2AndCrc(buf);
        }

        private void SoftReset(byte[] command)
        {
            this.Log(LogLevel.Noisy, "Performing soft reset");
            Reset();
        }

        private void ClearStatus(byte[] command)
        {
            this.Log(LogLevel.Noisy, "Clearing status");
        }

        private void Status(byte[] command)
        {
            // TODO - use the register type facilities for this?
            this.Log(LogLevel.Noisy, "Reading status Register");
            var res = new byte[] {0, 0};
            Enqueue2AndCrc(res);
        }

        private void ReadSerialNumber(byte[] command)
        {
            // TODO - use the register type facilities for this?
            this.Log(LogLevel.Noisy, "Reading serial number");
            var res = new byte[] {(byte) (serialNumber >> 8), (byte) (serialNumber & 0xFF)};
            Enqueue2AndCrc(res);
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

        private uint humidity = 0;
        private uint temperature = 0;
        private uint serialNumber = 0;

        private readonly I2CCommandManager<Action<byte[]>> commands;
        private readonly Queue<byte> outputBuffer;

        // TODO - made these up
        private const decimal MaxHumidity = 100;
        private const decimal MinHumidity = 0;
        private const decimal MaxTemperature = 85;
        private const decimal MinTemperature = -40;
    }
}
