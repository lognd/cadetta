using System.Linq;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Props;
using Cadetta.SolidWorks.Read;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Props
{
    public class ConfigPropertyTests
    {

        private static FakePropertySource Source()
        {
            var s = new FakePropertySource { ActiveConfiguration = "Default" };
            s.Names.AddRange(new[] { "Default", "Heavy" });
            return s.With(null, "Mass", "2.5")
                .With("Default", "Rev", "3")
                .With("Heavy", "Rev", "9", "\"SW-Rev\"");
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Props/ConfigPropertyReader.cs::ConfigPropertyReader.Collect kind=unit
        [Fact]
        public void Collect_keeps_file_level_and_active_configuration_properties_only()
        {
            var snap = ConfigPropertyReader.Collect(Source(), Log.Null());
            Assert.Equal("Default", snap.Active);
            Assert.Equal(new[] { "Default", "Heavy" }, snap.Configurations);
            Assert.Equal(new[] { "Mass", "Rev" }, snap.Properties.Select(p => p.Name));
            Assert.Null(snap.Properties[0].Config);
            Assert.Equal("Default", snap.Properties[1].Config);
            Assert.DoesNotContain(snap.Properties, p => p.Value == "9");
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Props/ConfigPropertyReader.cs::ConfigPropertyReader.Collect kind=unit
        [Fact]
        public void Collect_without_an_active_configuration_reads_file_level_only()
        {
            var s = new FakePropertySource().With(null, "Mass", "1").With("Default", "Rev", "3");
            var snap = ConfigPropertyReader.Collect(s, Log.Null());
            Assert.Null(snap.Active);
            Assert.Single(snap.Properties);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Props/ConfigPropertyReader.cs::ConfigPropertyReader.Apply kind=unit
        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.Build kind=unit
        [Fact]
        public void Active_configuration_and_properties_become_observed_parameters()
        {
            var doc = new NativeDocument { Path = "C:\\w\\Rail.SLDPRT" };
            ConfigPropertyReader.Apply(ConfigPropertyReader.Collect(Source(), Log.Null()), doc);
            var b = new ObservedIrBuilder(doc.Path, Log.Null());
            b.Add(doc);
            var root = b.Build().Observed!.Value;
            Assert.Equal("Default", root.GetProperty("active_config").GetString());
            Assert.Equal(new[] { "Default", "Heavy" },
                root.GetProperty("configurations").EnumerateArray().Select(e => e.GetString()).ToArray());
            var rev = root.GetProperty("bindings").EnumerateArray()
                .First(x => x.GetProperty("path").GetString()!.EndsWith(".Rev")).GetProperty("locators")[0];
            Assert.Equal("property", rev.GetProperty("kind").GetString());
            Assert.Equal("Default", rev.GetProperty("config").GetString());
            Assert.Equal(2, root.GetProperty("components")[0].GetProperty("members").GetArrayLength());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.Build kind=unit
        [Fact]
        public void Document_without_configurations_omits_the_keys()
        {
            var b = new ObservedIrBuilder("a.SLDPRT", Log.Null());
            b.Add(new NativeDocument { Path = "a.SLDPRT" });
            var root = b.Build().Observed!.Value;
            Assert.False(root.TryGetProperty("active_config", out _));
            Assert.False(root.TryGetProperty("configurations", out _));
        }
    }
}
