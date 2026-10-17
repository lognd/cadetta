// frob:ticket 01M54EDNCXH7QFEEGJQNAE1M1N
using System.Collections.Generic;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.AddIn.Commands;
using Cadetta.SolidWorks.Frames;
using Cadetta.SolidWorks.Logging;
using Xunit;

namespace Cadetta.SolidWorks.Commands.Tests
{
    internal sealed class FakeRefs : IComponentReferences
    {
        public Dictionary<string, string?> Values = new Dictionary<string, string?>();
        public string? Refuse;
        public string? GetReference(string instance) => Values.TryGetValue(instance, out var v) ? v : null;
        public bool SetReference(string instance, string reference)
        {
            if (instance == Refuse) return false;
            Values[instance] = reference;
            return true;
        }
    }

    public class ArrayAssignTests
    {
        private static readonly ProjectInfo Project = new ProjectInfo("/p", "/p/cadetta.toml", new List<AdapterRef>());

        private static string Row(string kind, string path, double x) =>
            "{\"kind\":\"" + kind + "\",\"path\":\"" + path + "\",\"transform\":[[1,0,0," + x.ToString(System.Globalization.CultureInfo.InvariantCulture) + "],[0,1,0,0],[0,0,1,0]]}";

        private const string Doc = "{\"schema\":\"cadetta.frames/1\",\"native_unit\":\"m\",\"precision\":6,\"frames\":["
            + "{\"kind\":\"datum\",\"path\":\"pkg::Mount.origin\",\"transform\":null},"
            + "{\"kind\":\"instance\",\"path\":\"pkg::Mount/hole.place\",\"transform\":null},"
            + "ROWS]}";

        private static string Frames(params string[] rows) => Doc.Replace("ROWS", string.Join(",", rows));

        private static string Standard() => Frames(
            Row("instance", "pkg::Mount/feet[1].place", 0.0),
            Row("instance", "pkg::Mount/feet[2].place", 0.1),
            Row("instance", "pkg::Mount/feet[3].place", 0.2),
            Row("instance", "pkg::Other/feet[1].place", 0.0));

        private static SelectedInstance Inst(string n, double x) => new SelectedInstance(n, new FrameTransform(FrameTransform.Identity.Rotation, x, 0, 0));

        private static CommandActions Actions(string json, int exit = 0)
            => new CommandActions(new CadettaCommands(new Runner(), null, Log.Null()), Log.Null(),
                new CadettaCli(new Runner { Output = new ProcessOutput(exit, json, exit == 0 ? "" : "boom") }, null, Log.Null()));

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.ArrayAssign.ArrayPrefix
        // frob:tests Cadetta.SolidWorks.Commands.ArrayAssign.ReferenceOf
        public void PathHelpers()
        {
            Assert.Equal("pkg::Mount/feet", ArrayAssign.ArrayPrefix("pkg::Mount/feet[2].place"));
            Assert.Equal("pkg::Mount/feet", ArrayAssign.ArrayPrefix("pkg::Mount/feet[12]"));
            Assert.Null(ArrayAssign.ArrayPrefix("pkg::Mount/hole.place"));
            Assert.Null(ArrayAssign.ArrayPrefix("pkg::Mount/feet[x].place"));
            Assert.Equal("feet[2]", ArrayAssign.ReferenceOf("pkg::Mount/feet[2].place"));
            Assert.Null(ArrayAssign.ReferenceOf("pkg::Mount"));
            Assert.Equal(5e-7, ArrayAssign.TieEpsilon(6), 12);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Client.FramesReport.Parse
        // frob:tests Cadetta.SolidWorks.AddIn.Client.FramesReport.ArrayCandidates
        public void ParsesAndFiltersToArray()
        {
            var report = FramesReport.Parse(Standard())!;
            Assert.Equal(6, report.Precision);
            var names = report.ArrayCandidates("pkg::Mount/feet").ConvertAll(c => c.Name);
            Assert.Equal(new[] { "pkg::Mount/feet[1].place", "pkg::Mount/feet[2].place", "pkg::Mount/feet[3].place" }, names);
            Assert.Null(FramesReport.Parse("not json"));
            Assert.Null(FramesReport.Parse("{\"schema\":\"other/1\"}"));
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Client.CadettaCli.Frames
        public void FramesClientBuildsArgsAndReportsFailure()
        {
            var runner = new Runner { Output = new ProcessOutput(0, Standard(), "") };
            var r = new CadettaCli(runner, null, Log.Null()).Frames(Project);
            Assert.True(r.IsOk);
            Assert.Equal(new[] { "frames", "--native-unit", "m", "--format", "json" }, runner.Args);
            runner.Output = new ProcessOutput(2, "", "bad");
            Assert.Contains("bad", new CadettaCli(runner, null, Log.Null()).Frames(Project).Error);
            runner.Output = new ProcessOutput(0, "{}", "");
            Assert.False(new CadettaCli(runner, null, Log.Null()).Frames(Project).IsOk);
            runner.Output = null;
            Assert.Equal("cannot start", new CadettaCli(runner, null, Log.Null()).Frames(Project).Error);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Commands.CommandActions.AssignArray
        // frob:tests Cadetta.SolidWorks.Commands.ArrayAssign.Assign
        public void AssignsByDistanceNotSelectionOrder()
        {
            var refs = new FakeRefs();
            var r = Actions(Standard()).AssignArray(Project, new FakeDoc(), new Probe(),
                new[] { Inst("Foot-9", 0.2), Inst("Foot-4", 0.0004), Inst("Foot-7", 0.1) }, refs, "pkg::Mount/feet[1].place");
            Assert.True(r.IsOk, r.Message);
            Assert.Equal("feet[3]", refs.Values["Foot-9"]);
            Assert.Equal("feet[1]", refs.Values["Foot-4"]);
            Assert.Equal("feet[2]", refs.Values["Foot-7"]);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Commands.CommandActions.AssignArray
        public void RefusesGateMissingPathAndCliFailure()
        {
            var refs = new FakeRefs();
            var sel = new[] { Inst("Foot-1", 0.0) };
            Assert.Equal("R009", Actions(Standard()).AssignArray(Project, new FakeDoc(), new Probe { Modal = true }, sel, refs, "pkg::Mount/feet[1].place").Code);
            Assert.False(Actions(Standard()).AssignArray(Project, new FakeDoc(), new Probe(), sel, refs, "pkg::Mount/hole.place").IsOk);
            Assert.False(Actions(Standard()).AssignArray(null, new FakeDoc(), new Probe(), sel, refs, "pkg::Mount/feet[1].place").IsOk);
            Assert.False(Actions(Standard(), 2).AssignArray(Project, new FakeDoc(), new Probe(), sel, refs, "pkg::Mount/feet[1].place").IsOk);
            Assert.Empty(refs.Values);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.ArrayAssign.Assign
        public void RefusesFarTiedDuplicateEmptyAllOrNothing()
        {
            var cands = new FramesReport { Frames = new List<FrameRow>() };
            var report = FramesReport.Parse(Standard())!;
            var c = report.ArrayCandidates("pkg::Mount/feet");
            var refs = new FakeRefs();
            // too far for the second instance: nothing written for the first either
            var far = ArrayAssign.Assign(new[] { Inst("A", 0.0), Inst("B", 5.0) }, c, 0.001, 5e-7, refs, Log.Null());
            Assert.False(far.IsOk);
            Assert.Empty(refs.Values);
            // two instances nearest the same frame
            var dup = ArrayAssign.Assign(new[] { Inst("A", 0.0), Inst("B", 0.0002) }, c, 0.001, 5e-7, refs, Log.Null());
            Assert.False(dup.IsOk);
            Assert.Empty(refs.Values);
            // equidistant between two frames
            Assert.False(ArrayAssign.Assign(new[] { Inst("A", 0.05) }, c, 0.06, 5e-7, refs, Log.Null()).IsOk);
            // nothing selected / no candidates
            Assert.False(ArrayAssign.Assign(new SelectedInstance[0], c, 0.001, 5e-7, refs, Log.Null()).IsOk);
            Assert.False(ArrayAssign.Assign(new[] { Inst("A", 0.0) }, cands.ArrayCandidates("x"), 0.001, 5e-7, refs, Log.Null()).IsOk);
            Assert.Empty(refs.Values);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.Commands.ArrayAssign.Assign
        public void UndoesWritesWhenSolidworksRefuses()
        {
            var report = FramesReport.Parse(Standard())!;
            var refs = new FakeRefs { Refuse = "B" };
            refs.Values["A"] = "old";
            var r = ArrayAssign.Assign(new[] { Inst("A", 0.0), Inst("B", 0.1) }, report.ArrayCandidates("pkg::Mount/feet"), 0.001, 5e-7, refs, Log.Null());
            Assert.False(r.IsOk);
            Assert.Equal("old", refs.Values["A"]);
        }
    }
}
