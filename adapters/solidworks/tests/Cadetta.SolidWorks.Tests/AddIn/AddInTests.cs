// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.AddIn.Logging;
using Cadetta.SolidWorks.AddIn.Pane;
using Cadetta.SolidWorks.Logging;
using Xunit;

namespace Cadetta.SolidWorks.AddIn.Tests
{
    internal sealed class FakeRunner : ICommandRunner
    {
        public ProcessOutput? Output;
        public string StartError = "";
        public string? LastExe;
        public IReadOnlyList<string> LastArgs = new List<string>();
        public string? LastDir;

        public ProcessOutput? Run(string exe, IReadOnlyList<string> args, string workingDir, out string startError)
        {
            LastExe = exe;
            LastArgs = args;
            LastDir = workingDir;
            startError = StartError;
            return Output;
        }
    }

    internal static class Fx
    {
        public const string Json = @"{""adapters"":[{""id"":""solidworks"",""names"":[
 {""path"":""pkg::Bracket.holes.pitch_x"",""kind"":""param"",""emit"":""40 mm""},
 {""path"":""pkg::Bracket.holes.pitch_y"",""kind"":""param"",""emit"":""25 mm""},
 {""path"":""pkg::Bracket.bend"",""kind"":""param"",""emit"":""90 deg""},
 {""path"":""pkg::Bracket.n"",""kind"":""param"",""emit"":""4""},
 {""path"":""pkg::Other.len"",""kind"":""param"",""emit"":""1 mm""}]}],""diagnostics"":[]}";

        public static string TempDir()
        {
            var d = Path.Combine(Path.GetTempPath(), "cdt-addin-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(d);
            return d;
        }

        public static (PaneModel Model, FakeRunner Runner, string Doc) Project(string json, int exit = 0)
        {
            var root = TempDir();
            File.WriteAllText(Path.Combine(root, "cadetta.toml"), "[[adapter]]\nname = \"solidworks\"\n");
            Directory.CreateDirectory(Path.Combine(root, "cad"));
            var runner = new FakeRunner { Output = new ProcessOutput(exit, json, "boom") };
            var model = new PaneModel(new CadettaCli(runner, null, Log.Null()), Log.Null());
            return (model, runner, Path.Combine(root, "cad", "Bracket.SLDPRT"));
        }
    }

    public class DiscoveryTests
    {
        // frob:tests ProjectDiscovery.ParseAdapters
        [Fact]
        public void ParseAdapters_FiltersSolidworksAndDefaultsId()
        {
            var toml = "[[adapter]]\nname = \"freecad\"\n[[adapter]]\nname = \"solidworks\" # c\nid = \"sw2\"\n[[adapter]]\nname = 'solidworks'\n";
            var a = ProjectDiscovery.ParseAdapters(toml);
            Assert.Equal(new[] { "sw2", "solidworks" }, a.Select(x => x.Id).ToArray());
        }

        // frob:tests ProjectDiscovery.Find
        [Fact]
        public void Find_WalksUpToManifest_AndMissingIsNull()
        {
            var (_, _, doc) = Fx.Project(Fx.Json);
            var p = ProjectDiscovery.Find(doc, Log.Null());
            Assert.NotNull(p);
            Assert.Equal(Path.GetDirectoryName(Path.GetDirectoryName(doc)), p!.Root.TrimEnd(Path.DirectorySeparatorChar));
            Assert.Null(ProjectDiscovery.Find(Path.Combine(Fx.TempDir(), "x.SLDPRT"), Log.Null()));
        }

        // frob:tests ProjectInfo.Pick
        [Fact]
        public void Pick_ChosenOrFirst()
        {
            var p = new ProjectInfo("r", "r/cadetta.toml", new[] { new AdapterRef("solidworks", "a"), new AdapterRef("solidworks", "b") });
            Assert.Equal("a", p.Pick(null)!.Id);
            Assert.Equal("b", p.Pick("b")!.Id);
        }
    }

    public class CliTests
    {
        private static CliResult<NamesAdapter> Run(FakeRunner r)
        {
            var p = new ProjectInfo("/proj", "/proj/cadetta.toml", new[] { new AdapterRef("solidworks", "solidworks") });
            return new CadettaCli(r, null, Log.Null()).Names(p, p.Adapters[0]);
        }

        // frob:tests CadettaCli.Names
        [Fact]
        public void Names_Success_RunsInProjectRoot()
        {
            var r = new FakeRunner { Output = new ProcessOutput(0, Fx.Json, "") };
            var res = Run(r);
            Assert.True(res.IsOk);
            Assert.Equal(5, res.Value!.Names.Count);
            Assert.Equal("cadetta", r.LastExe);
            Assert.Equal(new[] { "names", "solidworks", "--format", "json" }, r.LastArgs.ToArray());
            Assert.Equal("/proj", r.LastDir);
        }

        // frob:tests CadettaCli.Names
        [Fact]
        public void Names_Failures()
        {
            Assert.False(Run(new FakeRunner { Output = new ProcessOutput(2, "", "bad") }).IsOk);
            Assert.False(Run(new FakeRunner { Output = new ProcessOutput(0, "not json", "") }).IsOk);
            Assert.False(Run(new FakeRunner { Output = null, StartError = "no exe" }).IsOk);
            Assert.False(Run(new FakeRunner { Output = new ProcessOutput(0, "{\"adapters\":[]}", "") }).IsOk);
        }

        // frob:tests NamesReport.Parse
        [Fact]
        public void NamesReport_Parse()
        {
            Assert.Null(NamesReport.Parse("{"));
            Assert.Equal("40 mm", NamesReport.Parse(Fx.Json)!.Adapters[0].Names[0].Emit);
        }

        // frob:tests ProcessCommandRunner.Run
        [Fact]
        public void ProcessRunner_RunsRealProcess_AndReportsStartFailure()
        {
            var runner = new ProcessCommandRunner(Log.Null());
            var ok = runner.Run("dotnet", new[] { "--version" }, Directory.GetCurrentDirectory(), out var err);
            Assert.NotNull(ok);
            Assert.Equal(0, ok!.ExitCode);
            Assert.Null(runner.Run("definitely-not-a-binary-xyz", new string[0], Directory.GetCurrentDirectory(), out var e2));
            Assert.NotEqual("", e2);
        }

        // frob:tests ProcessCommandRunner.JoinArgs
        [Fact]
        public void JoinArgs_QuotesSpaces()
        {
            Assert.Equal("a \"b c\"", ProcessCommandRunner.JoinArgs(new[] { "a", "b c" }));
        }
    }

    public class PaneTests
    {
        // frob:tests Quantity.Parse
        [Fact]
        public void Quantity_Parse()
        {
            Assert.Equal(QuantityKind.Length, Quantity.Parse("40 mm").Kind);
            Assert.Equal(0.04, Quantity.Parse("40 mm").Si, 9);
            Assert.Equal(QuantityKind.Angle, Quantity.Parse("180 deg").Kind);
            Assert.Equal(Math.PI, Quantity.Parse("180 deg").Si, 9);
            Assert.Equal(QuantityKind.Count, Quantity.Parse("4").Kind);
            Assert.Equal(QuantityKind.Unknown, Quantity.Parse("abc").Kind);
            Assert.Equal(QuantityKind.Unknown, Quantity.Parse(null).Kind);
        }

        // frob:tests PathFilter.ComponentOf
        // frob:tests PathFilter.ComponentSegment
        [Fact]
        public void Component_Extraction()
        {
            Assert.Equal("Bracket", PathFilter.ComponentOf(@"C:\x\Bracket.SLDPRT"));
            Assert.Equal("Bracket", PathFilter.ComponentSegment("pkg::Bracket.holes.pitch_x"));
            Assert.Equal("Bracket", PathFilter.ComponentSegment("pkg::Bracket/x"));
        }

        // frob:tests PathFilter.ForComponent
        // frob:tests PathFilter.ForSelection
        [Fact]
        public void Selection_SortsByNearestValueOfSameKind()
        {
            var rows = NamesReport.Parse(Fx.Json)!.Adapters[0].Names;
            var comp = PathFilter.ForComponent(rows, "Bracket");
            Assert.Equal(4, comp.Count);
            var near = PathFilter.ForSelection(comp, new SelectedDimension(QuantityKind.Length, 0.026));
            Assert.Equal(new[] { "pkg::Bracket.holes.pitch_y", "pkg::Bracket.holes.pitch_x" }, near.Select(r => r.Path).ToArray());
            var ang = PathFilter.ForSelection(comp, new SelectedDimension(QuantityKind.Angle, 1.5));
            Assert.Equal("pkg::Bracket.bend", Assert.Single(ang).Path);
            Assert.Equal(4, PathFilter.ForSelection(comp, null).Count);
        }

        // frob:tests PaneModel.Load
        // frob:tests PaneModel.View
        [Fact]
        public void Model_LoadsComponentRows()
        {
            var (m, r, doc) = Fx.Project(Fx.Json);
            Assert.True(m.Load(doc));
            Assert.Equal(4, m.Rows.Count);
            Assert.Equal(Path.GetDirectoryName(Path.GetDirectoryName(doc)), r.LastDir!.TrimEnd(Path.DirectorySeparatorChar));
            Assert.Equal(2, m.View(new SelectedDimension(QuantityKind.Length, 0.04)).Count);
        }

        // frob:tests PaneModel.Load
        [Fact]
        public void Model_ReportsStates()
        {
            var (m, _, doc) = Fx.Project(Fx.Json, exit: 1);
            Assert.False(m.Load(null));
            Assert.False(m.Load(Path.Combine(Fx.TempDir(), "A.SLDPRT")));
            Assert.Contains("cadetta.toml", m.Status);
            Assert.False(m.Load(doc));
            Assert.Empty(m.Rows);
        }
    }

    public class LogTests
    {
        // frob:tests AddInLog.Open
        // frob:tests AddInLog.PathUnder
        [Fact]
        public void Open_WritesFilteredLog()
        {
            var dir = Fx.TempDir();
            var log = AddInLog.Open(dir, "info");
            log.Info("hello-info");
            log.Debug("hidden-debug");
            var path = AddInLog.PathUnder(dir);
            Assert.True(File.Exists(path));
            var text = ReadShared(path);
            Assert.Contains("hello-info", text);
            Assert.DoesNotContain("hidden-debug", text);
            Assert.Equal(Log.Null().GetType(), AddInLog.Open(null, null).GetType());
        }

        // frob:tests AddInLog.OpenDefault
        [Fact]
        public void OpenDefault_UsesEnvironment()
        {
            var dir = Fx.TempDir();
            var oldApp = Environment.GetEnvironmentVariable("LOCALAPPDATA");
            var oldLog = Environment.GetEnvironmentVariable("CADETTA_LOG");
            try
            {
                Environment.SetEnvironmentVariable("LOCALAPPDATA", dir);
                Environment.SetEnvironmentVariable("CADETTA_LOG", "info");
                AddInLog.OpenDefault().Info("env-line");
                Assert.Contains("env-line", ReadShared(AddInLog.PathUnder(dir)));
            }
            finally
            {
                Environment.SetEnvironmentVariable("LOCALAPPDATA", oldApp);
                Environment.SetEnvironmentVariable("CADETTA_LOG", oldLog);
            }
        }

        private static string ReadShared(string path)
        {
            using (var fs = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            using (var sr = new StreamReader(fs))
            {
                return sr.ReadToEnd();
            }
        }
    }
}
