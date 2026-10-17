using System.Collections.Generic;
using Cadetta.SolidWorks.Push;
using Cadetta.SolidWorks.Read;
using Cadetta.SolidWorks.Regen;
using Cadetta.SolidWorks.Save;
using Cadetta.SolidWorks.Session;

namespace Cadetta.SolidWorks.Transcripts.Tests
{
    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>IPushDocument that records each call against an inner document, or replays it from the tape.</summary>
    internal sealed class TapePushDocument : IPushDocument
    {
        private readonly Tape _t;
        private readonly IPushDocument? _in;

        public TapePushDocument(Tape t, IPushDocument? inner) { _t = t; _in = inner; }

        public string Path => _t.Do("Doc.Path", new object?[0], () => _in!.Path);
        public bool ReadOnly => _t.Do("Doc.ReadOnly", new object?[0], () => _in!.ReadOnly);
        public bool UserDirty => _t.Do("Doc.UserDirty", new object?[0], () => _in!.UserDirty);
        public bool AdapterModified => _t.Do("Doc.AdapterModified", new object?[0], () => _in!.AdapterModified);
        public double? GetGlobal(string name) => _t.Do("Doc.GetGlobal", new object?[] { name }, () => _in!.GetGlobal(name));
        public string? GetGlobalExpression(string name) => _t.Do("Doc.GetGlobalExpression", new object?[] { name }, () => _in!.GetGlobalExpression(name));
        public bool SetGlobal(string name, double value) => _t.Do("Doc.SetGlobal", new object?[] { name, value }, () => _in!.SetGlobal(name, value));
        public bool DeleteGlobal(string name) => _t.Do("Doc.DeleteGlobal", new object?[] { name }, () => _in!.DeleteGlobal(name));
        public DimensionInfo? GetDimension(string name) => _t.Do("Doc.GetDimension", new object?[] { name }, () => _in!.GetDimension(name));
        public bool SetDimensionValue(string name, double value) => _t.Do("Doc.SetDimensionValue", new object?[] { name, value }, () => _in!.SetDimensionValue(name, value));
        public bool SetDimensionLink(string name, string? global) => _t.Do("Doc.SetDimensionLink", new object?[] { name, global }, () => _in!.SetDimensionLink(name, global));
        public bool RestoreEquation(string lhs, string? rhs) => _t.Do("Doc.RestoreEquation", new object?[] { lhs, rhs }, () => _in!.RestoreEquation(lhs, rhs));
        public bool SetDimensionTolerance(string name, ToleranceSpec tolerance) => _t.Do("Doc.SetDimensionTolerance", new object?[] { name, tolerance }, () => _in!.SetDimensionTolerance(name, tolerance));
        public string? GetProperty(string name) => _t.Do("Doc.GetProperty", new object?[] { name }, () => _in!.GetProperty(name));
        public bool SetProperty(string name, string text) => _t.Do("Doc.SetProperty", new object?[] { name, text }, () => _in!.SetProperty(name, text));
        public bool DeleteProperty(string name) => _t.Do("Doc.DeleteProperty", new object?[] { name }, () => _in!.DeleteProperty(name));
        public double[]? GetFrame(string name) => _t.Do("Doc.GetFrame", new object?[] { name }, () => _in!.GetFrame(name));
        public bool SetFrame(string name, double[] transform, bool create) => _t.Do("Doc.SetFrame", new object?[] { name, transform, create }, () => _in!.SetFrame(name, transform, create));
        public bool DeleteFrame(string name) => _t.Do("Doc.DeleteFrame", new object?[] { name }, () => _in!.DeleteFrame(name));
        public bool Rebuild() => _t.Do("Doc.Rebuild", new object?[0], () => _in!.Rebuild());
    }

    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>ISaveTarget that records or replays the save-step calls.</summary>
    internal sealed class TapeSaveTarget : ISaveTarget
    {
        private readonly Tape _t;
        private readonly ISaveTarget? _in;

        public TapeSaveTarget(Tape t, ISaveTarget? inner) { _t = t; _in = inner; }

        public string Path => _t.Do("Target.Path", new object?[0], () => _in!.Path);
        public bool Modified => _t.Do("Target.Modified", new object?[0], () => _in!.Modified);
        public bool RebuildFailed => _t.Do("Target.RebuildFailed", new object?[0], () => _in!.RebuildFailed);
        public string? Save() => _t.Do("Target.Save", new object?[0], () => _in!.Save());
    }

    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>IPushHandle over a taped document and save target.</summary>
    internal sealed class TapePushHandle : IPushHandle
    {
        private readonly Tape _t;
        private readonly IPushHandle? _in;

        public TapePushHandle(Tape t, IPushHandle? inner)
        {
            _t = t;
            _in = inner;
            Document = new TapePushDocument(t, inner?.Document);
            Target = new TapeSaveTarget(t, inner?.Target);
        }

        public IPushDocument Document { get; }
        public ISaveTarget Target { get; }
        public void Settle(bool modified, bool rebuildFailed) => _t.Do("Handle.Settle", new object?[] { modified, rebuildFailed }, () => _in!.Settle(modified, rebuildFailed));
        public void Dispose() => _t.Do("Handle.Dispose", new object?[0], () => _in!.Dispose());
    }

    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>IPushHost that records or replays Open.</summary>
    internal sealed class TapePushHost : IPushHost
    {
        private readonly Tape _t;
        private readonly IPushHost? _in;

        public TapePushHost(Tape t, IPushHost? inner) { _t = t; _in = inner; }

        public IPushHandle Open(string file)
        {
            IPushHandle? h = null;
            _t.Do("Host.Open", new object?[] { file }, () => { h = _in!.Open(file); return true; });
            return new TapePushHandle(_t, h);
        }
    }

    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>IRegenDocument that records or replays the rebuild.</summary>
    internal sealed class TapeRegenDocument : IRegenDocument
    {
        private readonly Tape _t;
        private readonly IRegenDocument? _in;

        public TapeRegenDocument(Tape t, IRegenDocument? inner) { _t = t; _in = inner; }

        public string Path => _t.Do("Regen.Path", new object?[0], () => _in!.Path);
        public bool ReadOnly => _t.Do("Regen.ReadOnly", new object?[0], () => _in!.ReadOnly);
        public bool UserDirty => _t.Do("Regen.UserDirty", new object?[0], () => _in!.UserDirty);
        public IReadOnlyList<RebuildError> Rebuild() => _t.Do<List<RebuildError>>("Regen.Rebuild", new object?[0], () => new List<RebuildError>(_in!.Rebuild()));
    }

    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>IRegenHandle over a taped document.</summary>
    internal sealed class TapeRegenHandle : IRegenHandle
    {
        private readonly Tape _t;
        private readonly IRegenHandle? _in;

        public TapeRegenHandle(Tape t, IRegenHandle? inner)
        {
            _t = t;
            _in = inner;
            Document = new TapeRegenDocument(t, inner?.Document);
        }

        public IRegenDocument Document { get; }
        public void Dispose() => _t.Do("RegenHandle.Dispose", new object?[0], () => _in!.Dispose());
    }

    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>IRegenHost that records or replays Open.</summary>
    internal sealed class TapeRegenHost : IRegenHost
    {
        private readonly Tape _t;
        private readonly IRegenHost? _in;

        public TapeRegenHost(Tape t, IRegenHost? inner) { _t = t; _in = inner; }

        public IRegenHandle Open(string file)
        {
            IRegenHandle? h = null;
            _t.Do("RegenHost.Open", new object?[] { file }, () => { h = _in!.Open(file); return true; });
            return new TapeRegenHandle(_t, h);
        }
    }

    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>INativeDocumentReader that records or replays the extracted document.</summary>
    internal sealed class TapeReader : INativeDocumentReader
    {
        private readonly Tape _t;
        private readonly INativeDocumentReader? _in;

        public TapeReader(Tape t, INativeDocumentReader? inner) { _t = t; _in = inner; }

        public NativeDocument Read(string path) => _t.Do("Reader.Read", new object?[] { path }, () => _in!.Read(path));
    }
}
