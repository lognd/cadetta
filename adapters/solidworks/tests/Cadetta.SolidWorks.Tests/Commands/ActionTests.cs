// frob:ticket 01M54BDDBT01RBBDS3AQ8ZNA4K
using System.Collections.Generic;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.AddIn.Commands;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;
using Xunit;

namespace Cadetta.SolidWorks.Commands.Tests
{
    public class ActionTests
    {
        private static readonly ProjectInfo Project = new ProjectInfo("/p", "/p/cadetta.toml", new List<AdapterRef>());

        private static FakeDoc Doc()
        {
            var doc = new FakeDoc();
            doc.Dims["D1@Sketch1"] = new DimensionInfo { Value = 0.04 };
            return doc;
        }

        private static CommandActions Actions(Runner runner) => new CommandActions(new CadettaCommands(runner, null, Log.Null()), Log.Null());

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Commands.CommandActions.Link
        public void LinkUsesMangledPathAndHonoursGate()
        {
            var actions = Actions(new Runner());
            var doc = Doc();
            var ok = actions.Link(doc, new Probe(), "D1@Sketch1", "pkg::A.pitch");
            Assert.True(ok.IsOk);
            Assert.Equal(NameMangler.Mangle("pkg::A.pitch"), doc.Dims["D1@Sketch1"].LinkedGlobal);
            var busy = actions.Link(Doc(), new Probe { Modal = true }, "D1@Sketch1", "pkg::A.pitch");
            Assert.Equal("R009", busy.Code);
            Assert.False(actions.Link(doc, new Probe(), null, "pkg::A.pitch").IsOk);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Commands.CommandActions.Unlink
        public void UnlinkClearsAndRefusesReadOnly()
        {
            var actions = Actions(new Runner());
            var doc = Doc();
            doc.Dims["D1@Sketch1"].LinkedGlobal = "G";
            Assert.True(actions.Unlink(doc, new Probe(), "D1@Sketch1").IsOk);
            Assert.Null(doc.Dims["D1@Sketch1"].LinkedGlobal);
            doc.ReadOnly = true;
            Assert.Equal("R009", actions.Unlink(doc, new Probe(), "D1@Sketch1").Code);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Commands.CommandActions.Sync
        public void SyncReportsAndClearsRunningFlag()
        {
            var runner = new Runner { Output = new ProcessOutput(0, "pushed 3\n", "") };
            var actions = Actions(runner);
            var r = actions.Sync(Project, "solidworks", true, new string[0]);
            Assert.True(r.IsOk);
            Assert.Equal("pushed 3", r.Message);
            Assert.False(actions.SyncRunning);
            runner.Output = new ProcessOutput(2, "", "R008");
            Assert.False(actions.Sync(Project, "solidworks", false, new string[0]).IsOk);
            Assert.False(actions.SyncRunning);
            Assert.False(actions.Sync(null, "solidworks", false, new string[0]).IsOk);
        }

        [Fact]
        // frob:tests Cadetta.SolidWorks.AddIn.Commands.CommandActions.Why
        public void WhyReturnsDerivationText()
        {
            var runner = new Runner { Output = new ProcessOutput(0, "from A\n", "") };
            var r = Actions(runner).Why(Project, "pkg::A.x");
            Assert.Equal("from A", r.Message);
            Assert.Equal(new[] { "why", "pkg::A.x", "--format", "json" }, runner.Args);
            Assert.False(Actions(runner).Why(Project, null).IsOk);
        }
    }
}
