using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Push;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    public class PushServiceTests
    {
        private static JsonElement J(string raw) => JsonDocument.Parse(raw).RootElement.Clone();

        private static ChangedParam P(string locator, string mode, string path, string value, string? refTo = null, DimTolerance? tol = null, string? kind = null) =>
            new ChangedParam { Locator = locator, Mode = mode, Path = path, Value = J(value), RefTo = refTo, Tolerance = tol, Kind = kind };

        private static PushOutcome Run(FakePushDocument doc, params ChangedParam[] changes) =>
            new PushService(Log.Null()).Push(doc, changes, new PushOptions());

        private static FakePushDocument WithDim(string name, double value = 5)
        {
            var doc = new FakePushDocument();
            doc.Dims[name] = new DimensionInfo { Value = value };
            return doc;
        }

        private static LocatorStatus St(PushOutcome o, string locator) => o.Statuses.Single(s => s.Locator == locator);

        // frob:tests PushService.Push
        [Fact]
        public void Several_locators_on_one_path_each_get_pushed()
        {
            var doc = WithDim("D1@Sketch1");
            doc.Props["Material"] = "old";
            var o = Run(doc,
                P("WIDTH", "value", "pkg::p.w", "\"6\""),
                P("D1@Sketch1", "value", "pkg::p.w", "\"8\""),
                P("Material", "value", "pkg::p.w", "\"steel\""));
            Assert.All(o.Statuses, s => Assert.Equal(PushStatus.Pushed, s.Status));
            Assert.Equal(6, doc.Globals["WIDTH"]);
            Assert.Equal(8, doc.Dims["D1@Sketch1"].Value);
            Assert.Equal("steel", doc.Props["Material"]);
            Assert.Equal(1, doc.Rebuilds);
        }

        // frob:ticket 01M5496CWTHTRP1RQ57THH1WF6
        // frob:tests PushService.Push
        [Fact]
        public void Wire_kind_property_creates_a_new_custom_property_without_prefix()
        {
            var doc = new FakePushDocument();
            var o = Run(doc, P("Supplier", "value", "pkg::p.s", "\"acme\"", kind: LocatorKind.Property));
            Assert.Equal(PushStatus.Pushed, St(o, "Supplier").Status);
            Assert.Equal("acme", doc.Props["Supplier"]);
            Assert.False(doc.Globals.ContainsKey("Supplier"));
        }

        // frob:ticket 01M5496CWTHTRP1RQ57THH1WF6
        // frob:tests PushService.Push
        [Fact]
        public void Wire_kind_wins_over_inference_and_a_prefix_is_still_stripped()
        {
            var doc = new FakePushDocument();
            doc.Props["WIDTH"] = "1";
            var o = Run(doc, P("global:WIDTH", "value", "pkg::p.w", "\"6\"", kind: LocatorKind.Global));
            Assert.Equal(PushStatus.Pushed, St(o, "global:WIDTH").Status);
            Assert.Equal(6, doc.Globals["WIDTH"]);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Ref_dimension_links_to_ref_to_global()
        {
            var doc = WithDim("D1@Boss");
            var o = Run(doc, P("D1@Boss", "ref", "p.t", "\"6\"", refTo: "THICK"));
            Assert.Equal("THICK", doc.Dims["D1@Boss"].LinkedGlobal);
            Assert.Equal(6, doc.Globals["THICK"]);
            Assert.Equal(PushStatus.Pushed, St(o, "D1@Boss").Status);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Ref_dimension_prefers_first_global_bound_to_the_path()
        {
            var doc = WithDim("D1@Boss");
            var o = Run(doc,
                P("D1@Boss", "ref", "p.t", "\"6\""),
                P("G_FIRST", "value", "p.t", "\"6\""),
                P("G_SECOND", "value", "p.t", "\"6\""));
            Assert.Equal("G_FIRST", doc.Dims["D1@Boss"].LinkedGlobal);
            Assert.Equal(PushStatus.Pushed, St(o, "G_FIRST").Status);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Ref_dimension_falls_back_to_the_mangled_path()
        {
            var doc = WithDim("D1@Boss");
            Run(doc, P("D1@Boss", "ref", "pkg::mod::Part.thick[2]", "\"6\""));
            Assert.Equal("Part__thick_2", doc.Dims["D1@Boss"].LinkedGlobal);
            Assert.Equal(6, doc.Globals["Part__thick_2"]);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Value_not_representable_at_precision_is_q004()
        {
            var doc = new FakePushDocument();
            var o = Run(doc, P("W", "value", "p.w", "{\"r\":\"1/3\",\"pi\":0}"));
            Assert.Equal(PushStatus.Rejected, St(o, "W").Status);
            Assert.Contains(o.Diagnostics, d => d.GetProperty("code").GetString() == "Q004");
            Assert.Empty(doc.Writes);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Unchanged_locator_is_skipped_and_only_changed_are_pushed()
        {
            var doc = new FakePushDocument();
            doc.Globals["A"] = 3;
            doc.Globals["B"] = 1;
            var o = Run(doc, P("A", "value", "p.a", "\"3\""), P("B", "value", "p.b", "\"2\""));
            Assert.Equal(PushStatus.Skipped, St(o, "A").Status);
            Assert.Equal(PushStatus.Pushed, St(o, "B").Status);
            Assert.Equal(new[] { "global:B" }, doc.Writes);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Nothing_changed_means_no_rebuild()
        {
            var doc = new FakePushDocument();
            doc.Globals["A"] = 3;
            Run(doc, P("A", "value", "p.a", "\"3\""));
            Assert.Equal(0, doc.Rebuilds);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Dirty_or_read_only_document_rejects_everything_with_r009()
        {
            var doc = new FakePushDocument { UserDirty = true };
            var o = Run(doc, P("A", "value", "p.a", "\"3\""), P("B", "value", "p.b", "\"2\""));
            Assert.All(o.Statuses, s => Assert.Equal(PushStatus.Rejected, s.Status));
            Assert.Contains(o.Diagnostics, d => d.GetProperty("code").GetString() == "R009");
            Assert.Empty(doc.Writes);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Driven_dimension_is_rejected_r010_and_missing_is_skipped_r003()
        {
            var doc = WithDim("D1@S");
            doc.Dims["D1@S"].Driven = true;
            var o = Run(doc, P("D1@S", "value", "p.a", "\"3\""), P("D9@S", "value", "p.b", "\"2\""));
            Assert.Equal(PushStatus.Rejected, St(o, "D1@S").Status);
            Assert.Equal(PushStatus.Skipped, St(o, "D9@S").Status);
            var codes = o.Diagnostics.Select(d => d.GetProperty("code").GetString()).ToList();
            Assert.Contains("R010", codes);
            Assert.Contains("R003", codes);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Value_push_to_a_linked_dimension_warns_r006_and_drops_the_link()
        {
            var doc = WithDim("D1@S");
            doc.Dims["D1@S"].LinkedGlobal = "G";
            var o = Run(doc, P("D1@S", "value", "p.a", "\"9\""));
            Assert.Null(doc.Dims["D1@S"].LinkedGlobal);
            Assert.Equal(9, doc.Dims["D1@S"].Value);
            Assert.Contains(o.Diagnostics, d => d.GetProperty("code").GetString() == "R006");
        }

        // frob:tests PushService.Push
        [Fact]
        public void Rebuild_failure_restores_values_and_marks_pushed_failed_r004()
        {
            var doc = new FakePushDocument { RebuildFailsAbove = 10 };
            doc.Globals["A"] = 2;
            var o = Run(doc, P("A", "value", "p.a", "\"50\""), P("NEW", "value", "p.n", "\"1\""));
            Assert.Equal(2, doc.Globals["A"]);
            Assert.False(doc.Globals.ContainsKey("NEW"));
            Assert.All(o.Statuses, s => Assert.Equal(PushStatus.Failed, s.Status));
            Assert.True(o.RebuildFailed);
            Assert.Contains(o.Diagnostics, d => d.GetProperty("code").GetString() == "R004");
        }

        // frob:tests PushService.Push
        [Fact]
        public void Rebuild_failure_message_names_the_failing_feature()
        {
            var doc = new FakePushDocument { RebuildFailsAbove = 10, FailingFeature = "Shell1" };
            doc.Globals["A"] = 2;
            var o = Run(doc, P("A", "value", "p.a", "\"50\""));
            var msg = o.Diagnostics.Single(d => d.GetProperty("code").GetString() == "R004").GetProperty("message").GetString();
            Assert.Contains("Shell1", msg);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Rebuild_failure_without_a_named_feature_keeps_the_plain_message()
        {
            var doc = new FakePushDocument { RebuildFailsAbove = 10 };
            doc.Globals["A"] = 2;
            var o = Run(doc, P("A", "value", "p.a", "\"50\""));
            var msg = o.Diagnostics.Single(d => d.GetProperty("code").GetString() == "R004").GetProperty("message").GetString();
            Assert.DoesNotContain("feature", msg);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Tolerance_is_applied_and_read_back()
        {
            var doc = WithDim("D1@S", 5);
            var tol = new DimTolerance { Form = "bilateral", Lower = "-0.1", Upper = "0.2" };
            var o = Run(doc, P("D1@S", "value", "p.a", "\"5\"", tol: tol));
            Assert.Equal(PushStatus.Pushed, St(o, "D1@S").Status);
            Assert.Equal(ToleranceKind.Bilateral, doc.Dims["D1@S"].Tolerance.Kind);
            Assert.Equal("bilateral", St(o, "D1@S").Tolerance!.Form);
        }

        // frob:tests PushService.Push
        [Fact]
        public void Missing_frame_is_r003_unless_datums_create()
        {
            var m = "[[1,0,0,10],[0,1,0,0],[0,0,1,0]]";
            var verify = new FakePushDocument();
            var o = Run(verify, P("CS1", "frame", "p.f", m));
            Assert.Equal(PushStatus.Skipped, St(o, "CS1").Status);
            Assert.Contains(o.Diagnostics, d => d.GetProperty("code").GetString() == "R003");

            var create = new FakePushDocument();
            var o2 = new PushService(Log.Null()).Push(create, new[] { P("CS1", "frame", "p.f", m) }, new PushOptions { CreateDatums = true });
            Assert.Equal(PushStatus.Pushed, St(o2, "CS1").Status);
            Assert.Equal(10, create.Frames["CS1"][3]);
        }
    }
}
