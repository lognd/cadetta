using System;
using System.Collections.Generic;
using Cadetta.SolidWorks.Frames;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Restore;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Frames
{
    internal sealed class FakeFrames : IFrameDocument, ISketchDocument, IRestorableDocument
    {
        public Dictionary<string, FrameReading> Frames = new Dictionary<string, FrameReading>();
        public Dictionary<string, SketchFacts> Sketches = new Dictionary<string, SketchFacts>();
        public Dictionary<string, double> Values = new Dictionary<string, double>();
        public bool Refuse;

        public FrameReading? ReadFrame(string locator) => Frames.TryGetValue(locator, out var f) ? f : null;

        public bool CreateOrUpdateCoordinateSystem(string name, FrameTransform metres)
        {
            if (Refuse) { return false; }
            Frames[name] = new FrameReading(FrameKind.CoordinateSystem, metres);
            return true;
        }

        public SketchFacts? ReadSketch(string dimensionLocator) => Sketches.TryGetValue(dimensionLocator, out var s) ? s : null;

        public double? GetValue(string locator) => Values.TryGetValue(locator, out var v) ? v : (double?)null;

        public bool SetValue(string locator, double value) { Values[locator] = value; return true; }

        public bool Rebuild() => true;
    }

    public class FrameTests
    {
        private static readonly Log L = Log.Null();

        private static FrameTransform Rz(double deg, double x, double y, double z)
        {
            var c = Math.Cos(deg * Math.PI / 180);
            var s = Math.Sin(deg * Math.PI / 180);
            return new FrameTransform(new[] { c, -s, 0, s, c, 0, 0, 0, 1.0 }, x, y, z);
        }

        // frob:tests FrameTransform.FromSwArray
        [Fact]
        public void SwArrayReadsAxesAsRows()
        {
            // API-SURFACE.md: 90 degrees about z.
            var a = new double[] { 0, 1, 0, -1, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0 };
            var t = FrameTransform.FromSwArray(a);
            Assert.True(t.Matches(Rz(90, 0, 0, 0), 6, false));
            Assert.Equal(a, t.ToSwArray().AsSpan(0, 16).ToArray());
        }

        // frob:tests FrameTransform.ToEulerXyz
        [Theory]
        [InlineData(0, 0, 0)]
        [InlineData(90, 0, 0)]
        [InlineData(35, 0, 0)]
        public void EulerRoundTripsRz(double deg, double a, double b)
        {
            var e = Rz(deg, 0, 0, 0).ToEulerXyz();
            Assert.Equal(deg * Math.PI / 180, e[2], 9);
            Assert.Equal(a, e[0], 9);
            Assert.Equal(b, e[1], 9);
        }

        // frob:tests FrameTransform.Matches
        [Fact]
        public void PlaneComparisonIgnoresInPlaneRotation()
        {
            Assert.False(Rz(30, 0, 0, 0).Matches(Rz(0, 0, 0, 0), 3, false));
            Assert.True(Rz(30, 0, 0, 0).Matches(Rz(0, 0, 0, 0), 3, true));
            Assert.False(Rz(0, 0.0006, 0, 0).Matches(Rz(0, 0, 0, 0), 3, true));
        }

        // frob:tests FrameReader.Read
        [Fact]
        public void ReaderReportsNativeUnitAndSkipsMissing()
        {
            var d = new FakeFrames();
            d.Frames["CS"] = new FrameReading(FrameKind.CoordinateSystem, Rz(0, 0.01, 0.02, 0.03));
            d.Frames["Front"] = new FrameReading(FrameKind.Plane, FrameTransform.Identity);
            var r = FrameReader.Read(d, "a.sldprt", new[] { "CS", "Front", "gone" }, 0.001, L);
            Assert.Equal(2, r.Count);
            Assert.Equal(10, r[0].Transform[0][3], 9);
            Assert.Equal(30, r[0].Transform[2][3], 9);
            Assert.True(r[1].PlaneOnly);
        }

        // frob:tests FrameWriter.Push
        [Fact]
        public void WriterCreatesThenSkipsThenMoves()
        {
            var d = new FakeFrames();
            var rows = new double[] { 1, 0, 0, 10, 0, 1, 0, 20, 0, 0, 1, 30 };
            Assert.Equal(PushStatus.Pushed, FrameWriter.Push(d, "CS", rows, 0.001, 3, L).Status);
            Assert.Equal(0.01, d.Frames["CS"].Transform.X, 9);
            Assert.Equal(PushStatus.Skipped, FrameWriter.Push(d, "CS", rows, 0.001, 3, L).Status);
            rows[3] = 11;
            Assert.Equal(PushStatus.Pushed, FrameWriter.Push(d, "CS", rows, 0.001, 3, L).Status);
            Assert.Equal(0.011, d.Frames["CS"].Transform.X, 9);
        }

        // frob:tests FrameWriter.Push
        [Fact]
        public void WriterFailsOnPlaneAndOnRefusal()
        {
            var d = new FakeFrames();
            d.Frames["Front"] = new FrameReading(FrameKind.Plane, FrameTransform.Identity);
            var rows = new double[] { 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0 };
            Assert.Equal(PushStatus.Failed, FrameWriter.Push(d, "Front", rows, 0.001, 3, L).Status);
            d.Refuse = true;
            Assert.Equal(PushStatus.Failed, FrameWriter.Push(d, "New", rows, 0.001, 3, L).Status);
        }

        // frob:tests CoordinateBinder.ToDimension
        [Theory]
        [InlineData(25.0, 1, 0.001, 0.025)]
        [InlineData(25.0, -1, 0.001, -0.025)]
        public void SignAndUnitAreApplied(double native, int sign, double unit, double expected)
        {
            Assert.Equal(expected, CoordinateBinder.ToDimension(native, sign, unit), 12);
            Assert.Equal(native, CoordinateBinder.FromDimension(expected, sign, unit), 9);
        }

        // frob:tests CoordinateBinder.ToDimension
        [Fact]
        public void BadSignIsAProgrammerError()
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => CoordinateBinder.ToDimension(1, 2, 1));
        }

        // frob:tests CoordinateBinder.Push
        [Fact]
        public void PushWritesSignedValueAndSkipsEqual()
        {
            var d = new FakeFrames();
            d.Values["D1@S"] = 0;
            Assert.Equal(PushStatus.Pushed, CoordinateBinder.Push(d, "D1@S", 25, -1, 0.001, null, L).Status);
            Assert.Equal(-0.025, d.Values["D1@S"], 12);
            Assert.Equal(PushStatus.Skipped, CoordinateBinder.Push(d, "D1@S", 25, -1, 0.001, null, L).Status);
            Assert.Equal(PushStatus.Failed, CoordinateBinder.Push(d, "none", 1, 1, 0.001, null, L).Status);
        }

        // frob:tests SketchObserver.Observe
        [Fact]
        public void CleanSketchReportsOriginAndPlane()
        {
            var d = new FakeFrames();
            d.Frames["Datum"] = new FrameReading(FrameKind.CoordinateSystem, Rz(0, 0, 0, 0.5));
            d.Sketches["D1@S"] = new SketchFacts(Rz(0, 0.3, 0.1, 0.5), SketchFacts.SketchOrigin);
            var o = SketchObserver.Observe(d, d, "a.sldprt", "D1@S", "Datum", L)!;
            Assert.True(o.OnDatumPlane);
            Assert.Equal(SketchFacts.SketchOrigin, o.MeasuredFrom);
        }

        // frob:tests SketchObserver.Observe
        [Fact]
        public void OffPlaneOrForeignOriginIsReportedForR007()
        {
            var d = new FakeFrames();
            d.Frames["Datum"] = new FrameReading(FrameKind.CoordinateSystem, Rz(0, 0, 0, 0.5));
            d.Sketches["Off@S"] = new SketchFacts(Rz(0, 0, 0, 0.6), SketchFacts.SketchOrigin);
            d.Sketches["Edge@S"] = new SketchFacts(Rz(0, 0, 0, 0.5), "edge");
            Assert.False(SketchObserver.Observe(d, d, "a", "Off@S", "Datum", L)!.OnDatumPlane);
            Assert.Equal("edge", SketchObserver.Observe(d, d, "a", "Edge@S", "Datum", L)!.MeasuredFrom);
            Assert.Null(SketchObserver.Observe(d, d, "a", "Nope", "Datum", L));
            Assert.Null(SketchObserver.Observe(d, d, "a", "Off@S", "Missing", L));
        }

        // frob:tests SketchObserver.OnPlane
        [Fact]
        public void TiltedSketchIsOffPlane()
        {
            var tilt = new FrameTransform(new double[] { 1, 0, 0, 0, 0, -1, 0, 1, 0 }, 0, 0, 0);
            Assert.False(SketchObserver.OnPlane(tilt, FrameTransform.Identity));
        }
    }
}
