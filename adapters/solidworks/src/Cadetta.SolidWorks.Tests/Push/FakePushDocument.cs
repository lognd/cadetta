using System.Collections.Generic;
using Cadetta.SolidWorks.Push;

namespace Cadetta.SolidWorks.Tests.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>In-memory IPushDocument for PushService tests; records writes and can fail the rebuild.</summary>
    internal sealed class FakePushDocument : IPushDocument
    {
        public Dictionary<string, double> Globals = new Dictionary<string, double>();
        public Dictionary<string, DimensionInfo> Dims = new Dictionary<string, DimensionInfo>();
        public Dictionary<string, string> Props = new Dictionary<string, string>();
        public Dictionary<string, double[]> Frames = new Dictionary<string, double[]>();
        public List<string> Writes = new List<string>();
        public int Rebuilds;
        public double RebuildFailsAbove = double.MaxValue;

        public string Path { get; set; } = "part.sldprt";
        public bool ReadOnly { get; set; }
        public bool UserDirty { get; set; }
        public bool AdapterModified { get; set; }

        public double? GetGlobal(string name) => Globals.TryGetValue(name, out var v) ? v : (double?)null;

        public bool SetGlobal(string name, double value)
        {
            Writes.Add("global:" + name);
            Globals[name] = value;
            return true;
        }

        public bool DeleteGlobal(string name) => Globals.Remove(name);

        public DimensionInfo? GetDimension(string name) => Dims.TryGetValue(name, out var d) ? d : null;

        public bool SetDimensionValue(string name, double value)
        {
            Writes.Add("dim:" + name);
            Dims[name].Value = value;
            return true;
        }

        public bool SetDimensionLink(string name, string? global)
        {
            Writes.Add("link:" + name);
            Dims[name].LinkedGlobal = global;
            return true;
        }

        public bool SetDimensionTolerance(string name, ToleranceSpec spec)
        {
            Writes.Add("tol:" + name);
            Dims[name].Tolerance = spec;
            return true;
        }

        public string? GetProperty(string name) => Props.TryGetValue(name, out var v) ? v : null;

        public bool SetProperty(string name, string value)
        {
            Writes.Add("prop:" + name);
            Props[name] = value;
            return true;
        }

        public bool DeleteProperty(string name) => Props.Remove(name);

        public double[]? GetFrame(string name) => Frames.TryGetValue(name, out var f) ? f : null;

        public bool SetFrame(string name, double[] m, bool create)
        {
            if (!Frames.ContainsKey(name) && !create) return false;
            Writes.Add("frame:" + name);
            Frames[name] = m;
            return true;
        }

        public bool DeleteFrame(string name) => Frames.Remove(name);

        public bool Rebuild()
        {
            Rebuilds++;
            foreach (var v in Globals.Values)
            {
                if (v > RebuildFailsAbove) return false;
            }
            return true;
        }
    }
}
