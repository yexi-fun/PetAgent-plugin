using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Web.Script.Serialization;
using Microsoft.Win32;
using LibreHardwareMonitor.Hardware;

namespace PetAgent.HardwareTemperature
{
    internal sealed class Reading
    {
        public string hardware { get; set; }
        public string hardwareType { get; set; }
        public string name { get; set; }
        public string sensorType { get; set; }
        public float? value { get; set; }
    }

    internal sealed class Response
    {
        public bool ok { get; set; }
        public string error { get; set; }
        public List<Reading> sensors { get; set; }
    }

    internal static class Program
    {
        private static readonly JavaScriptSerializer Json = new JavaScriptSerializer();

        private static void EnsurePawnIo()
        {
            using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO"))
            {
                if (key != null)
                    return;
            }
            var setup = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "PawnIO_setup.exe");
            if (!File.Exists(setup))
                return;
            try
            {
                using (var process = Process.Start(new ProcessStartInfo
                {
                    FileName = setup,
                    Arguments = "-install",
                    UseShellExecute = true,
                    Verb = "runas",
                    WindowStyle = ProcessWindowStyle.Hidden
                }))
                {
                    process?.WaitForExit(30000);
                }
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine("[hardware-temperature-helper] PawnIO install skipped: " + ex.Message);
            }
        }

        private static void Collect(IEnumerable<IHardware> hardwareList, List<Reading> readings)
        {
            foreach (var hardware in hardwareList)
            {
                try
                {
                    hardware.Update();
                    foreach (var sensor in hardware.Sensors)
                    {
                        if (!sensor.Value.HasValue || float.IsNaN(sensor.Value.Value) || float.IsInfinity(sensor.Value.Value))
                            continue;
                        readings.Add(new Reading
                        {
                            hardware = hardware.Name,
                            hardwareType = hardware.HardwareType.ToString(),
                            name = sensor.Name,
                            sensorType = sensor.SensorType.ToString(),
                            value = sensor.Value.Value
                        });
                    }
                    if (hardware.SubHardware != null && hardware.SubHardware.Length > 0)
                        Collect(hardware.SubHardware, readings);
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine("[hardware-temperature-helper] " + ex.GetType().Name + ": " + ex.Message);
                }
            }
        }

        private static Response ReadSensors(Computer computer)
        {
            var readings = new List<Reading>();
            Collect(computer.Hardware, readings);
            return new Response { ok = true, sensors = readings };
        }

        public static int Main()
        {
            var computer = new Computer
            {
                IsCpuEnabled = true,
                IsGpuEnabled = true,
                IsMotherboardEnabled = true,
                IsStorageEnabled = false,
                IsMemoryEnabled = false,
                IsNetworkEnabled = false,
                IsControllerEnabled = false,
                IsPsuEnabled = false
            };
            try
            {
                EnsurePawnIo();
                computer.Open();
                string line;
                while ((line = Console.ReadLine()) != null)
                {
                    if (line.Trim().Equals("read", StringComparison.OrdinalIgnoreCase)
                        || line.IndexOf("\"cmd\"", StringComparison.OrdinalIgnoreCase) >= 0)
                    {
                        Console.WriteLine(Json.Serialize(ReadSensors(computer)));
                        Console.Out.Flush();
                    }
                }
                return 0;
            }
            catch (Exception ex)
            {
                Console.WriteLine(Json.Serialize(new Response
                {
                    ok = false,
                    error = ex.GetType().Name + ": " + ex.Message,
                    sensors = new List<Reading>()
                }));
                Console.Out.Flush();
                return 1;
            }
            finally
            {
                try { computer.Close(); } catch { }
            }
        }
    }
}
