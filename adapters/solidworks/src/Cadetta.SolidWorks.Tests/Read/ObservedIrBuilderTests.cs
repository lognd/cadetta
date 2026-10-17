using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Read;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Read
{
    public class ObservedIrBuilderTests
    {
        private const string Src = "adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs";

        private static NativeQuantity[] Identity() => new[]
        {
            NativeQuantity.Of("1", ""), NativeQuantity.Of("0", ""), NativeQuantity.Of("0", ""), NativeQuantity.Of("10", "mm"),
            NativeQuantity.Of("0", ""), NativeQuantity.Of("1", ""), NativeQuantity.Of("0", ""), NativeQuantity.Of("20", "mm"),
            NativeQuantity.Of("0", ""), NativeQuantity.Of("0", ""), NativeQuantity.Of("1", ""), NativeQuantity.Of("30", "mm"),
        };

        private static NativeDocument Sample()
        {
            var d = new NativeDocument { Path = "C:\\w\\Rail Part.SLDPRT" };
            d.Globals.Add(new NativeGlobal { Name = "Width", Value = NativeQuantity.Of("10") });
            d.Globals.Add(new NativeGlobal { Name = "Half", Value = NativeQuantity.Of("5"), Expression = "\"Width\" / 2" });
            d.Dimensions.Add(new NativeDimension
            {
                Locator = "D1@Boss-Extrude1",
                Value = NativeQuantity.Of("12.5"),
                Expression = "\"Width\"",
                Tolerance = new Protocol.DimTolerance { Form = "symmetric", T = "0.1" },
            });
            d.Dimensions.Add(new NativeDimension
            {
                Locator = "D2@Sketch1",
                Value = NativeQuantity.Of("7", null, "driven dimension: value is a measurement, not an input"),
            });
            d.Properties.Add(new NativeProperty { Name = "Mass", Value = "2.5" });
            d.Properties.Add(new NativeProperty { Name = "Description", Value = "bracket" });
            d.Properties.Add(new NativeProperty { Name = "Rev", Config = "Default", Value = "3" });
            d.Frames.Add(new NativeFrame { Name = "Site", Transform = Identity().ToList() });
            d.Instances.Add(new NativeInstance { Component = "bracket-1", Document = "bracket.sldprt", Reference = "bracket", Transform = Identity().ToList() });
            return d;
        }

        private static JsonElement Observe(NativeDocument d, out Protocol.ReadResult result)
        {
            var b = new ObservedIrBuilder(d.Path, Log.Null());
            b.Add(d);
            result = b.Build();
            return result.Observed!.Value;
        }

        private static JsonElement Member(JsonElement root, string suffix) =>
            root.GetProperty("components")[0].GetProperty("members").EnumerateArray()
                .First(m => m.GetProperty("path").GetString()!.EndsWith("." + suffix));

        private static JsonElement Binding(JsonElement root, string suffix) =>
            root.GetProperty("bindings").EnumerateArray().First(b => b.GetProperty("path").GetString()!.EndsWith("." + suffix));

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.Add kind=unit
        [Fact]
        public void Globals_and_equations_carry_unit_assumed_from_the_native_unit()
        {
            var root = Observe(Sample(), out _);
            Assert.Equal("observed", root.GetProperty("flavour").GetString());
            Assert.Equal("rail_part", root.GetProperty("package").GetProperty("name").GetString());
            var width = Member(root, "Width").GetProperty("value");
            Assert.Equal("mm", width.GetProperty("unit_assumed").GetString());
            Assert.Equal("1/100", width.GetProperty("nominal").GetProperty("r").GetString());
            Assert.Equal(1, width.GetProperty("dim").GetProperty("m").GetInt32());
            Assert.True(width.GetProperty("exact").GetBoolean());
            var half = Member(root, "Half");
            Assert.Equal("\"Width\" / 2", half.GetProperty("expr").GetString());
            Assert.Equal("1/200", half.GetProperty("value").GetProperty("nominal").GetProperty("r").GetString());
            var loc = Binding(root, "Width").GetProperty("locators")[0];
            Assert.Equal("global", loc.GetProperty("kind").GetString());
            Assert.Equal("mm", loc.GetProperty("native_unit").GetString());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.Add kind=unit
        [Fact]
        public void Dimension_is_observed_with_its_locator_equation_and_tolerance()
        {
            var root = Observe(Sample(), out var result);
            var m = Member(root, "D1_Boss_Extrude1");
            Assert.Equal("\"Width\"", m.GetProperty("expr").GetString());
            Assert.Equal("D1@Boss-Extrude1", m.GetProperty("provenance").GetProperty("imported").GetProperty("native_name").GetString());
            Assert.Equal("Rail Part.SLDPRT", m.GetProperty("provenance").GetProperty("imported").GetProperty("file").GetString());
            var loc = Binding(root, "D1_Boss_Extrude1").GetProperty("locators")[0];
            Assert.Equal("dimension", loc.GetProperty("kind").GetString());
            Assert.Equal("D1@Boss-Extrude1", loc.GetProperty("locator").GetString());
            var tol = result.Tolerances!.First(t => t.Locator == "D1@Boss-Extrude1").Tolerance!;
            Assert.Equal("symmetric", tol.Form);
            Assert.Equal("0.1", tol.T);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.Add kind=unit
        [Fact]
        public void Properties_are_observed_with_kind_property_and_config()
        {
            var root = Observe(Sample(), out _);
            var mass = Binding(root, "Mass").GetProperty("locators")[0];
            Assert.Equal("property", mass.GetProperty("kind").GetString());
            Assert.False(mass.TryGetProperty("config", out _));
            var rev = Binding(root, "Rev").GetProperty("locators")[0];
            Assert.Equal("Default", rev.GetProperty("config").GetString());
            var text = Member(root, "Description").GetProperty("value");
            Assert.False(text.GetProperty("exact").GetBoolean());
            Assert.Contains("text", text.GetProperty("reason").GetString());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.Add kind=unit
        [Fact]
        public void Driven_value_is_inexact_with_a_reason()
        {
            var root = Observe(Sample(), out _);
            var v = Member(root, "D2_Sketch1").GetProperty("value");
            Assert.False(v.GetProperty("exact").GetBoolean());
            Assert.Contains("driven", v.GetProperty("reason").GetString());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.Build kind=unit
        [Fact]
        public void Frames_and_instances_carry_3x4_transforms_and_reference()
        {
            var root = Observe(Sample(), out _);
            var f = root.GetProperty("frames")[0];
            Assert.Equal("Site", f.GetProperty("name").GetString());
            Assert.Equal(3, f.GetProperty("transform").GetArrayLength());
            Assert.Equal(4, f.GetProperty("transform")[0].GetArrayLength());
            Assert.Equal("1/100", f.GetProperty("transform")[0][3].GetProperty("nominal").GetProperty("r").GetString());
            Assert.Equal("1/1", f.GetProperty("transform")[1][1].GetProperty("nominal").GetProperty("r").GetString());
            var i = root.GetProperty("instances")[0];
            Assert.Equal("bracket", i.GetProperty("reference").GetString());
            Assert.Equal("bracket.sldprt", i.GetProperty("document").GetString());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.Ident kind=unit
        [Fact]
        public void Identifiers_are_path_safe_and_collisions_get_suffixes()
        {
            Assert.Equal("D1_Sketch1", ObservedIrBuilder.Ident("D1@Sketch1"));
            Assert.Equal("_1x", ObservedIrBuilder.Ident("1x"));
            var d = new NativeDocument { Path = "a.sldprt" };
            d.Globals.Add(new NativeGlobal { Name = "D1@S", Value = NativeQuantity.Of("1") });
            d.Globals.Add(new NativeGlobal { Name = "D1_S", Value = NativeQuantity.Of("2") });
            var root = Observe(d, out _);
            Assert.Equal("a::Observed.D1_S_2", Member(root, "D1_S_2").GetProperty("path").GetString());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.TryRational kind=unit
        [Fact]
        public void Long_binary64_text_is_not_exact()
        {
            var d = new NativeDocument { Path = "a.sldprt" };
            d.Globals.Add(new NativeGlobal { Name = "X", Value = NativeQuantity.Of("0.30000000000000004") });
            d.Globals.Add(new NativeGlobal { Name = "A", Value = NativeQuantity.Of("90", "deg") });
            var root = Observe(d, out _);
            var x = Member(root, "X").GetProperty("value");
            Assert.False(x.GetProperty("exact").GetBoolean());
            Assert.True(x.GetProperty("nominal").TryGetProperty("f64", out _));
            var a = Member(root, "A").GetProperty("value");
            Assert.Equal("1/2", a.GetProperty("nominal").GetProperty("r").GetString());
            Assert.Equal(1, a.GetProperty("nominal").GetProperty("pi").GetInt32());
            Assert.False(a.TryGetProperty("unit_assumed", out _));
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ReadService.cs::ReadService.Read kind=unit
        [Fact]
        public void ReadService_reads_through_the_reader()
        {
            var reader = new FakeNativeDocumentReader().With(Sample());
            var svc = new ReadService(reader, null, Log.Null());
            var r = svc.Read("C:\\w\\Rail Part.SLDPRT");
            Assert.Equal(1, reader.Reads);
            Assert.NotNull(r.Observed);
            Assert.Equal(2, r.Tolerances!.Count);
        }
    }
}
