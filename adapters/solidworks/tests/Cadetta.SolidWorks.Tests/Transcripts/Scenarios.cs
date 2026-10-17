using System;
using System.Collections.Generic;
using System.IO;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;
using Cadetta.SolidWorks.Read;
using Cadetta.SolidWorks.Regen;
using Cadetta.SolidWorks.Save;
using Cadetta.SolidWorks.Server;
using Cadetta.SolidWorks.Session;
using Cadetta.SolidWorks.Tests.Push;

namespace Cadetta.SolidWorks.Transcripts.Tests
{
    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>One named exchange: a wire request plus the in-memory native state it is recorded against.</summary>
    internal sealed class Scenario
    {
        public Scenario(string name, string request, Action<Fakes> arrange)
        {
            Name = name;
            Request = request;
            Arrange = arrange;
        }

        /// <summary>Fixture base name, e.g. push-ok.</summary>
        public string Name { get; }

        /// <summary>The request line; {file} stands for the temp file path.</summary>
        public string Request { get; }

        /// <summary>Sets up the fakes a recording run drives.</summary>
        public Action<Fakes> Arrange { get; }
    }

    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>The in-memory native side a recording run drives; replay never touches it.</summary>
    internal sealed class Fakes
    {
        public FakePushDocument Doc = new FakePushDocument { AdapterModified = true };
        public RegenDoc Regen = new RegenDoc();
        public FakeNativeDocumentReader Reader = new FakeNativeDocumentReader();
    }

    internal sealed class RegenDoc : IRegenDocument
    {
        public string Path { get; set; } = "";
        public bool ReadOnly { get; set; }
        public bool UserDirty { get; set; }
        public List<RebuildError> Errors = new List<RebuildError>();
        public IReadOnlyList<RebuildError> Rebuild() => Errors;
    }

    internal sealed class RecTarget : ISaveTarget
    {
        public string Path { get; set; } = "";
        public bool Modified { get; set; }
        public bool RebuildFailed { get; set; }
        public string? Save() => null;
    }

    internal sealed class RecPushHandle : IPushHandle
    {
        public RecPushHandle(FakePushDocument doc, string file) { Doc = doc; Doc.Path = file; Tgt = new RecTarget { Path = file }; }
        public FakePushDocument Doc;
        public RecTarget Tgt;
        public IPushDocument Document => Doc;
        public ISaveTarget Target => Tgt;
        public void Settle(bool modified, bool rebuildFailed) { Tgt.Modified = modified; Tgt.RebuildFailed = rebuildFailed; }
        public void Dispose() { }
    }

    internal sealed class RecPushHost : IPushHost
    {
        private readonly Fakes _f;
        public RecPushHost(Fakes f) { _f = f; }
        public IPushHandle Open(string file) => new RecPushHandle(_f.Doc, file);
    }

    internal sealed class RecRegenHandle : IRegenHandle
    {
        public RecRegenHandle(RegenDoc d) { Document = d; }
        public IRegenDocument Document { get; }
        public void Dispose() { }
    }

    internal sealed class RecRegenHost : IRegenHost
    {
        private readonly Fakes _f;
        public RecRegenHost(Fakes f) { _f = f; }
        public IRegenHandle Open(string file) { _f.Regen.Path = file; return new RecRegenHandle(_f.Regen); }
    }

    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>The scenario catalogue and the driver that records or replays one through RpcServer.Handle.</summary>
    internal static class Scenarios
    {
        private static string Push(string paramsJson) =>
            "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"push\",\"params\":{\"file\":\"{file}\",\"params\":" + paramsJson + ",\"session\":\"s\"}}";

        private static string Value(string locator, string value) =>
            "{\"locator\":\"" + locator + "\",\"mode\":\"value\",\"path\":\"p.x\",\"value\":" + value + "}";

        private const string RegenReq = "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"regen\",\"params\":{\"session\":\"s\",\"targets\":[\"{file}\"]}}";
        private const string ReadReq = "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"read\",\"params\":{\"source\":{\"files\":[\"rail.sldprt\"]}}}";

        /// <summary>Every scenario; each has a fixture of the same name next to this file.</summary>
        public static readonly IReadOnlyList<Scenario> All = new List<Scenario>
        {
            new Scenario("push-ok",
                Push("[" + Value("global:WIDTH", "\"6\"") + "," + Value("dimension:D1@Boss-Extrude1", "\"12\"") + "," + Value("property:Rev", "\"B\"") + "]"),
                f =>
                {
                    f.Doc.Globals["WIDTH"] = 5;
                    f.Doc.Dims["D1@Boss-Extrude1"] = new DimensionInfo { Value = 10 };
                    f.Doc.Props["Rev"] = "A";
                }),
            new Scenario("push-q004",
                Push("[" + Value("global:WIDTH", "{\"nominal\":{\"r\":\"1/2\",\"pi\":1}}") + "]"),
                f => f.Doc.Globals["WIDTH"] = 5),
            new Scenario("push-r004-rebuild",
                Push("[" + Value("global:WIDTH", "\"500\"") + "]"),
                f =>
                {
                    f.Doc.Globals["WIDTH"] = 5;
                    f.Doc.RebuildFailsAbove = 100;
                }),
            new Scenario("push-r006-equation",
                Push("[" + Value("global:WIDTH", "\"6\"") + "]"),
                f =>
                {
                    f.Doc.Globals["WIDTH"] = 5;
                    f.Doc.GlobalExpressions["WIDTH"] = "\"A\" * 2";
                }),
            new Scenario("push-r009-readonly",
                Push("[" + Value("global:WIDTH", "\"6\"") + "]"),
                f =>
                {
                    f.Doc.Globals["WIDTH"] = 5;
                    f.Doc.ReadOnly = true;
                }),
            new Scenario("push-r010-driven",
                Push("[" + Value("dimension:D1@Boss-Extrude1", "\"12\"") + "]"),
                f => f.Doc.Dims["D1@Boss-Extrude1"] = new DimensionInfo { Value = 10, Driven = true }),
            new Scenario("regen-clean", RegenReq, f => { }),
            new Scenario("regen-r004", RegenReq,
                f => f.Regen.Errors.Add(new RebuildError { Feature = "Boss-Extrude1", Code = 4, Message = "feature error code 4" })),
            new Scenario("regen-r009-dirty", RegenReq, f => f.Regen.UserDirty = true),
            new Scenario("read-observed", ReadReq, f => f.Reader.With(RailPart())),
            new Scenario("read-missing", ReadReq, f => { }),
        };

        private static NativeDocument RailPart()
        {
            var d = new NativeDocument { Path = "rail.sldprt", ActiveConfig = "Default" };
            d.Configurations.Add("Default");
            d.Globals.Add(new NativeGlobal { Name = "Width", Value = NativeQuantity.Of("12", "mm") });
            d.Globals.Add(new NativeGlobal { Name = "Half", Value = NativeQuantity.Of("6", "mm"), Expression = "\"Width\" / 2" });
            d.Dimensions.Add(new NativeDimension { Locator = "D1@Boss-Extrude1", Value = NativeQuantity.Of("12", "mm"), Expression = "\"Width\"" });
            d.Dimensions.Add(new NativeDimension { Locator = "D2@Sketch1", Value = NativeQuantity.Of("30", "mm", "driven (reference) dimension") });
            d.Properties.Add(new NativeProperty { Name = "Description", Value = "Rail" });
            return d;
        }

        /// <summary>Runs <paramref name="s"/> through RpcServer.Handle over taped hosts; recording drives fakes, replay answers from the tape.</summary>
        public static string Run(Scenario s, Tape tape)
        {
            var f = new Fakes();
            if (tape.Recording) s.Arrange(f);
            var log = new Log(new StringWriter(), LogLevel.Trace);
            var rec = tape.Recording;
            var server = new RpcServer(
                new FakeSwSession(),
                log,
                new ReadService(new TapeReader(tape, rec ? f.Reader : null), null, log),
                new TapePushHost(tape, rec ? new RecPushHost(f) : null),
                new TapeRegenHost(tape, rec ? new RecRegenHost(f) : null));
            var request = tape.Expand(s.Request);
            var response = server.Handle(request) ?? "";
            tape.Exchange(request, response);
            return response;
        }
    }
}
