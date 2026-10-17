using System;
using Cadetta.SolidWorks.Push;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Push
{
    public sealed class DimensionUnitsTests
    {
        [Theory]
        [InlineData(2, DimensionUnitKind.Length)]
        [InlineData(3, DimensionUnitKind.Angle)]
        [InlineData(13, DimensionUnitKind.Count)]
        [InlineData(6, DimensionUnitKind.Length)]
        public void KindOfMapsSwDimensionType(int sw, DimensionUnitKind want) => Assert.Equal(want, DimensionUnits.KindOf(sw));

        [Fact]
        public void CountIsNotScaled()
        {
            Assert.Equal(5.0, DimensionUnits.ToNative(5.0, DimensionUnitKind.Count));
            Assert.Equal(5.0, DimensionUnits.ToSystem(5.0, DimensionUnitKind.Count));
        }

        [Fact]
        public void LengthAndAngleScale()
        {
            Assert.Equal(25.0, DimensionUnits.ToNative(0.025, DimensionUnitKind.Length), 9);
            Assert.Equal(0.025, DimensionUnits.ToSystem(25.0, DimensionUnitKind.Length), 9);
            Assert.Equal(90.0, DimensionUnits.ToNative(Math.PI / 2, DimensionUnitKind.Angle), 9);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/DimensionUnits.cs::DimensionUnits.ToQuantity kind=unit
        [Fact]
        public void ToQuantityKeepsCountsUnscaledAndUnitless()
        {
            var q = DimensionUnits.ToQuantity(5.0, DimensionUnitKind.Count);
            Assert.Equal("5", q.Text);
            Assert.Equal("", q.Unit);
            var len = DimensionUnits.ToQuantity(0.025, DimensionUnitKind.Length);
            Assert.Equal("25", len.Text);
            Assert.Null(len.Unit);
            Assert.Equal("rad", DimensionUnits.ToQuantity(1.5, DimensionUnitKind.Angle).Unit);
        }
    }
}
