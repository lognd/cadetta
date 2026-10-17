using Cadetta.SolidWorks.Probe;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Probe
{
    public class ApiProbeReportTests
    {
        private static readonly ApiProbeResult[] Results =
        {
            new ApiProbeResult("equation manager", "IEquationMgr.Add2", true, "count=1"),
            new ApiProbeResult("tolerance", "a|b", false, "line1\nline2"),
        };

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Probe/ApiProbeReport.cs::ApiProbeReport.ToMarkdown kind=unit
        [Fact]
        public void Markdown_has_one_row_per_result_with_escaping()
        {
            var md = ApiProbeReport.ToMarkdown(Results);
            Assert.Contains("| equation manager | `IEquationMgr.Add2` | ok | count=1 |", md);
            Assert.Contains("| tolerance | `a\\|b` | FAIL | line1 line2 |", md);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Probe/ApiProbeReport.cs::ApiProbeReport.AllOk kind=unit
        [Fact]
        public void AllOk_is_false_when_any_probe_failed()
        {
            Assert.False(ApiProbeReport.AllOk(Results));
            Assert.True(ApiProbeReport.AllOk(new[] { Results[0] }));
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Probe/FakeApiProbe.cs::FakeApiProbe.Run kind=unit
        [Fact]
        public void Fake_probe_returns_its_canned_results()
        {
            Assert.Same(Results, new FakeApiProbe(Results).Run("ignored"));
        }
    }
}
