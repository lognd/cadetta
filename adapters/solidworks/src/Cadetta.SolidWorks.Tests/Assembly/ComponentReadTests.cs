using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Assembly;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Read;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Assembly
{
    // frob:ticket 01M4SWME04D18722NZS7TE82RN
    public class ComponentReadTests
    {
        private const string Naming = "adapters/solidworks/src/Cadetta.SolidWorks.Core/Assembly/ComponentNaming.cs";
        private const string Facts = "adapters/solidworks/src/Cadetta.SolidWorks.Core/Assembly/ComponentFacts.cs";
        private const string Builder = "adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs";

        private static System.Collections.Generic.List<NativeQuantity> Id() =>
            new[] { "1", "0", "0", "0", "0", "1", "0", "0", "0", "0", "1", "0" }.Select(x => NativeQuantity.Of(x, x == "0" ? "mm" : "")).ToList();

        private static NativeDocument Asm()
        {
            var d = new NativeDocument { Path = "C:\\w\\Cart.SLDASM" };
            d.Instances.Add(new NativeInstance { Component = "Sub-1/Wheel-2", Document = "C:\\w\\Hub.SLDPRT", Transform = Id() });
            d.Instances.Add(new NativeInstance { Component = "Frame-1", Document = "C:\\w\\Frame.SLDPRT", Transform = Id() });
            d.Dimensions.Add(new NativeDimension { Locator = "D1@Sketch1@Wheel.SLDPRT", Value = NativeQuantity.Of("5"), Component = "Wheel<2>" });
            return d;
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Assembly/ComponentNaming.cs::ComponentNaming.Instance kind=unit
        [Theory]
        [InlineData("Bracket-1", "Bracket<1>")]
        [InlineData("Sub-1/Wheel-2", "Wheel<2>")]
        [InlineData("Plain", null)]
        public void InstanceNameIsAngleForm(string name2, string? want) => Assert.Equal(want, ComponentNaming.Instance(name2));

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Assembly/ComponentNaming.cs::ComponentNaming.FileOf kind=unit
        [Fact]
        public void FileOfHandlesBothSlashes() => Assert.Equal("a.SLDPRT", ComponentNaming.FileOf("C:\\x\\a.SLDPRT"));

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Assembly/ComponentFacts.cs::ComponentFacts.From kind=unit
        [Fact]
        public void FactsCarryHeldFile()
        {
            var facts = ComponentFacts.From(Asm(), Log.Null());
            Assert.Equal(2, facts.Count);
            Assert.Equal("Cart.SLDASM", facts[0].File);
            Assert.Equal("Wheel<2>", facts[0].Locator);
            Assert.Equal("Hub.SLDPRT", facts[0].ComponentFile);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs::ObservedIrBuilder.Add kind=unit
        [Fact]
        public void BuilderEmitsComponentsAndLocatorField()
        {
            var b = new ObservedIrBuilder("C:\\w\\Cart.SLDASM", Log.Null());
            b.Add(Asm());
            var r = b.Build();
            Assert.Equal(2, r.Components!.Count);
            var raw = r.Observed!.Value.GetRawText().Replace(" ", "").Replace("\\u003C", "<").Replace("\\u003E", ">");
            Assert.Contains("\"component\":\"Wheel<2>\"", raw);
        }
    }
}
