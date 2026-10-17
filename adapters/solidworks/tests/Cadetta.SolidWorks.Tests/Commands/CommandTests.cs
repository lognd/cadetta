// frob:ticket 01M4VB7VXXHJ6TKHF6J2STC1FE
using System.Collections.Generic;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.AddIn.Commands;
using Cadetta.SolidWorks.Busy;
using Cadetta.SolidWorks.Commands;
using Cadetta.SolidWorks.Frames;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;
using Xunit;

namespace Cadetta.SolidWorks.Commands.Tests
{
    internal sealed class FakeDoc : IPushDocument
    {
        public Dictionary<string, double> Globals = new Dictionary<string, double>();
        public Dictionary<string, DimensionInfo> Dims = new Dictionary<string, DimensionInfo>();
        public bool RefuseLink;
        public string Path => "part.SLDPRT";
        public bool ReadOnly { get; set; }
        public bool UserDirty => false;
        public System.Collections.Generic.IReadOnlyList<Cadetta.SolidWorks.Regen.RebuildError> LastRebuildFailures { get; } = new System.Collections.Generic.List<Cadetta.SolidWorks.Regen.RebuildError>();
        public bool AdapterModified => false;
        public double? GetGlobal(string name) => Globals.TryGetValue(name, out var v) ? v : (double?)null;
        public string? GetGlobalExpression(string name) => null;
        public DimensionUnitKind LastGlobalKind;
        public bool SetGlobal(string name, double value, DimensionUnitKind kind = DimensionUnitKind.Length) { Globals[name] = value; LastGlobalKind = kind; return true; }
        public bool DeleteGlobal(string name) => Globals.Remove(name);
        public DimensionInfo? GetDimension(string name) => Dims.TryGetValue(name, out var d) ? d : null;
        public bool SetDimensionValue(string name, double value) => true;
        public bool SetDimensionLink(string name, string? global)
        {
            if (RefuseLink) return false;
            Dims[name].LinkedGlobal = global;
            return true;
        }
        public bool RestoreEquation(string lhs, string? rhs) => true;
        public bool SetDimensionTolerance(string name, ToleranceSpec tolerance) => true;
        public string? GetProperty(string name) => null;
        public bool SetProperty(string name, string text) => true;
        public bool DeleteProperty(string name) => true;
        public double[]? GetFrame(string name) => null;
        public bool SetFrame(string name, double[] transform, bool create) => true;
        public bool DeleteFrame(string name) => true;
        public bool Rebuild() => true;
    }

    internal sealed class Probe : IBusyProbe
    {
        public bool Modal;
        public bool Command;
        public bool ModalDialogOpen() => Modal;
        public bool CommandActive() => Command;
    }

    internal sealed class Runner : ICommandRunner
    {
        public ProcessOutput? Output = new ProcessOutput(0, "{}", "");
        public IReadOnlyList<string> Args = new List<string>();
        public ProcessOutput? Run(string exe, IReadOnlyList<string> args, string workingDir, out string startError)
        {
            Args = args;
            startError = "cannot start";
            return Output;
        }
    }

    public class LinkTests
    {
        private static FakeDoc Doc(DimensionInfo? d = null)
        {
            var doc = new FakeDoc();
            doc.Dims["D1@Sketch1"] = d ?? new DimensionInfo { Value = 0.04 };
            return doc;
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.LinkCommands.Link
        public void LinkCreatesGlobalAndEquation()
        {
            var doc = Doc();
            var r = LinkCommands.Link(doc, "D1@Sketch1", "PITCH", Log.Null());
            Assert.True(r.IsOk);
            Assert.Equal(0.04, doc.Globals["PITCH"]);
            Assert.Equal("PITCH", doc.Dims["D1@Sketch1"].LinkedGlobal);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.LinkCommands.Link
        public void LinkOfCountDimensionCreatesUnitlessGlobal()
        {
            var doc = Doc(new DimensionInfo { Value = 4, Kind = DimensionUnitKind.Count });
            Assert.True(LinkCommands.Link(doc, "D1@Sketch1", "N", Log.Null()).IsOk);
            Assert.Equal(DimensionUnitKind.Count, doc.LastGlobalKind);
            Assert.Equal(4.0, doc.Globals["N"]);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.LinkCommands.Link
        public void LinkRefusesDrivenAndUnknownAndRollsBackGlobal()
        {
            Assert.Equal("R010", LinkCommands.Link(Doc(new DimensionInfo { Driven = true }), "D1@Sketch1", "P", Log.Null()).Code);
            Assert.False(LinkCommands.Link(Doc(), "D9@Nope", "P", Log.Null()).IsOk);
            var doc = Doc();
            doc.RefuseLink = true;
            Assert.False(LinkCommands.Link(doc, "D1@Sketch1", "P", Log.Null()).IsOk);
            Assert.Empty(doc.Globals);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.LinkCommands.Link
        public void LinkIsIdempotentAndRefusesAnotherLink()
        {
            var doc = Doc(new DimensionInfo { LinkedGlobal = "P" });
            Assert.True(LinkCommands.Link(doc, "D1@Sketch1", "P", Log.Null()).IsOk);
            Assert.False(LinkCommands.Link(doc, "D1@Sketch1", "Q", Log.Null()).IsOk);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.LinkCommands.Unlink
        public void UnlinkRemovesEquation()
        {
            var doc = Doc(new DimensionInfo { LinkedGlobal = "P" });
            Assert.True(LinkCommands.Unlink(doc, "D1@Sketch1", Log.Null()).IsOk);
            Assert.Null(doc.Dims["D1@Sketch1"].LinkedGlobal);
            Assert.True(LinkCommands.Unlink(doc, "D1@Sketch1", Log.Null()).IsOk);
            Assert.False(LinkCommands.Unlink(doc, "D2@x", Log.Null()).IsOk);
        }
    }

    public class GateTests
    {
        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.DocumentGate.Check
        public void RefusesRunningSyncReadOnlyAndBusy()
        {
            var log = Log.Null();
            Assert.Null(DocumentGate.Check(new FakeDoc(), new Probe(), false, log));
            Assert.Equal("R009", DocumentGate.Check(new FakeDoc(), new Probe(), true, log)!.Code);
            Assert.Equal("R009", DocumentGate.Check(new FakeDoc { ReadOnly = true }, new Probe(), false, log)!.Code);
            Assert.Equal("R009", DocumentGate.Check(new FakeDoc(), new Probe { Modal = true }, false, log)!.Code);
            Assert.Equal("R009", DocumentGate.Check(new FakeDoc(), new Probe { Command = true }, false, log)!.Code);
        }
    }

    public class MatcherTests
    {
        private static FrameCandidate C(string n, double x) =>
            new FrameCandidate(n, new FrameTransform(FrameTransform.Identity.Rotation, x, 0, 0));

        private static readonly FrameTransform Target = new FrameTransform(FrameTransform.Identity.Rotation, 0.1, 0, 0);

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.FrameMatcher.Nearest
        public void PicksUniqueNearest()
        {
            var r = FrameMatcher.Nearest(new[] { C("a-1", 0.0), C("a-2", 0.1005), C("a-3", 0.3) }, Target, 0.01, 1e-6, Log.Null(), out var chosen);
            Assert.True(r.IsOk);
            Assert.Equal("a-2", chosen!.Name);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.FrameMatcher.Nearest
        public void RefusesTieEmptyAndFar()
        {
            Assert.False(FrameMatcher.Nearest(new[] { C("a", 0.09), C("b", 0.11) }, Target, 0.05, 1e-6, Log.Null(), out var c1).IsOk);
            Assert.Null(c1);
            Assert.False(FrameMatcher.Nearest(new FrameCandidate[0], Target, 0.05, 1e-6, Log.Null(), out _).IsOk);
            Assert.False(FrameMatcher.Nearest(new[] { C("a", 0.9) }, Target, 0.05, 1e-6, Log.Null(), out _).IsOk);
        }
    }

    public class CliTests
    {
        private static readonly ProjectInfo Project = new ProjectInfo("/p", "/p/cadetta.toml", new List<AdapterRef>());

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Commands.CadettaCommands.Bind
        public void BindBuildsArgs()
        {
            var runner = new Runner();
            var r = new CadettaCommands(runner, null, Log.Null()).Bind(Project, "solidworks", "pkg::A.x", "D1@Sketch1", "dimension", null, "arm-1");
            Assert.True(r.IsOk);
            Assert.Equal(new[] { "bind", "pkg::A.x", "--adapter", "solidworks", "--locator", "D1@Sketch1", "--kind", "dimension", "--component", "arm-1", "--format", "json" }, runner.Args);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Commands.CadettaCommands.Unbind
        public void UnbindUsesRemove()
        {
            var runner = new Runner();
            new CadettaCommands(runner, null, Log.Null()).Unbind(Project, "solidworks", "pkg::A.x", "D1@Sketch1");
            Assert.Equal("--remove", runner.Args[1]);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Commands.CadettaCommands.Sync
        public void SyncDryRunOnlyAndFailure()
        {
            var runner = new Runner();
            var cmds = new CadettaCommands(runner, null, Log.Null());
            Assert.True(cmds.Sync(Project, "solidworks", true, new[] { "pkg::A.*" }).IsOk);
            Assert.Equal(new[] { "sync", "solidworks", "--dry-run", "--only", "pkg::A.*", "--format", "json" }, runner.Args);
            runner.Output = new ProcessOutput(2, "", "R008");
            Assert.Contains("R008", cmds.Sync(Project, "solidworks", false, new string[0]).Error);
            runner.Output = null;
            Assert.Equal("cannot start", cmds.Why(Project, "pkg::A.x").Error);
        }
    }
}
