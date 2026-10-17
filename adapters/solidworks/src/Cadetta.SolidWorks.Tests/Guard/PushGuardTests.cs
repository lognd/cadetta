using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Guard;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Push;
using Cadetta.SolidWorks.Tests.Push;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Guard
{
    // frob:ticket 01M4SWMDS8MDHBS4T9JX1R3K6N
    public class PushGuardTests
    {
        private static ChangedParam P(string locator, string value) =>
            new ChangedParam { Locator = locator, Mode = "value", Path = "p.a", Value = JsonDocument.Parse(value).RootElement.Clone() };

        private static PushOutcome Run(FakePushDocument doc, params ChangedParam[] changes) =>
            new PushService(Log.Null()).Push(doc, changes, new PushOptions());

        private static string[] Codes(PushOutcome o) => o.Diagnostics.Select(d => d.GetProperty("code").GetString() ?? "").ToArray();

        // frob:tests PushGuard.ForDimension
        [Fact]
        public void Read_only_dimension_is_rejected_r010()
        {
            var v = PushGuard.ForDimension("D1@S", new DimensionInfo { ReadOnly = true });
            Assert.Equal(GuardAction.Reject, v.Action);
            Assert.Equal("R010", v.Code);
        }

        // frob:tests PushGuard.ForDimension
        [Fact]
        public void Driven_suppressed_and_conflict_are_rejected_r010()
        {
            foreach (var info in new[] { new DimensionInfo { Driven = true }, new DimensionInfo { Suppressed = true }, new DimensionInfo { Conflict = true } })
            {
                Assert.Equal("R010", PushGuard.ForDimension("D1@S", info).Code);
            }
        }

        // frob:tests PushGuard.ForDimension
        [Fact]
        public void Plain_dimension_is_allowed_and_expression_is_listed_r006()
        {
            Assert.Equal(GuardAction.Allow, PushGuard.ForDimension("D1@S", new DimensionInfo()).Action);
            var v = PushGuard.ForDimension("D1@S", new DimensionInfo { Expression = "\"A\" * 2" });
            Assert.Equal(GuardAction.ReplaceExpression, v.Action);
            Assert.Equal("R006", v.Code);
            Assert.Equal("\"A\" * 2", v.OldExpression);
            Assert.Contains("\"A\" * 2", v.Reason);
        }

        // frob:tests PushGuard.ForeignExpression
        [Theory]
        [InlineData("6mm", null)]
        [InlineData("-1.5e2deg", null)]
        [InlineData("\"WIDTH\"", null)]
        [InlineData("\"WIDTH\" * 2", "\"WIDTH\" * 2")]
        [InlineData("sin(\"A\")", "sin(\"A\")")]
        public void Foreign_expression_ignores_literals_and_links(string rhs, string? expected)
        {
            Assert.Equal(expected, PushGuard.ForeignExpression(rhs));
        }

        // frob:tests PushGuard.ForGlobal
        [Fact]
        public void Global_with_an_expression_replaces_it_r006()
        {
            Assert.Equal(GuardAction.Allow, PushGuard.ForGlobal("G", null).Action);
            Assert.Equal("R006", PushGuard.ForGlobal("G", "\"B\" * 2").Code);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Push_to_a_read_only_dimension_reports_r010_and_writes_nothing()
        {
            var doc = new FakePushDocument();
            doc.Dims["D1@S"] = new DimensionInfo { Value = 5, ReadOnly = true };
            var o = Run(doc, P("D1@S", "\"9\""));
            Assert.Equal(PushStatus.Rejected, o.Statuses.Single().Status);
            Assert.Contains("R010", Codes(o));
            Assert.Empty(doc.Writes);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Push_to_an_equation_driven_dimension_lists_it_r006_then_sets_the_value()
        {
            var doc = new FakePushDocument();
            doc.Dims["D1@S"] = new DimensionInfo { Value = 5, Expression = "\"A\" * 2" };
            var o = Run(doc, P("D1@S", "\"9\""));
            Assert.Equal(PushStatus.Pushed, o.Statuses.Single().Status);
            Assert.Equal(9, doc.Dims["D1@S"].Value);
            Assert.Null(doc.Dims["D1@S"].Expression);
            var r006 = o.Diagnostics.Single(d => d.GetProperty("code").GetString() == "R006");
            Assert.Contains("\"A\" * 2", r006.GetProperty("message").GetString());
            Assert.True(doc.Writes.IndexOf("eq:D1@S") < doc.Writes.IndexOf("dim:D1@S"));
        }

        // frob:tests PushService.Push
        [Fact]
        public void Push_to_a_global_defined_by_an_expression_warns_r006()
        {
            var doc = new FakePushDocument();
            doc.Globals["G"] = 4;
            doc.GlobalExpressions["G"] = "\"B\" * 2";
            var o = Run(doc, P("G", "\"7\""));
            Assert.Equal(7, doc.Globals["G"]);
            Assert.Contains("R006", Codes(o));
        }

        // frob:tests PushService.Push
        [Fact]
        public void Rebuild_failure_restores_a_replaced_dimension_expression()
        {
            var doc = new FakePushDocument { RebuildFailsAbove = 10 };
            doc.Globals["BIG"] = 1;
            doc.Dims["D1@S"] = new DimensionInfo { Value = 5, Expression = "\"A\" * 2" };
            var o = Run(doc, P("D1@S", "\"9\""), P("BIG", "\"50\""));
            Assert.Contains("R004", Codes(o));
            Assert.Equal("\"A\" * 2", doc.Dims["D1@S"].Expression);
        }
    }
}
