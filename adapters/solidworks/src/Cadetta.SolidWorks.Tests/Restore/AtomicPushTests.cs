using System.Collections.Generic;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Restore;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Restore
{
    public class AtomicPushTests
    {
        private sealed class Doc : IRestorableDocument
        {
            public Dictionary<string, double> Values = new Dictionary<string, double>();
            public int Rebuilds;
            public double FailAbove = double.MaxValue;
            public bool RefuseRestore;
            public double? GetValue(string l) => Values.TryGetValue(l, out var v) ? v : (double?)null;
            public bool SetValue(string l, double v)
            {
                if (RefuseRestore && v <= FailAbove) { return false; }
                Values[l] = v;
                return true;
            }
            public bool Rebuild()
            {
                Rebuilds++;
                foreach (var v in Values.Values) { if (v > FailAbove) { return false; } }
                return true;
            }
        }

        private static List<KeyValuePair<string, double>> Change(string k, double v) =>
            new List<KeyValuePair<string, double>> { new KeyValuePair<string, double>(k, v) };

        [Fact]
        public void Failed_rebuild_restores_pre_push_values()
        {
            var doc = new Doc { FailAbove = 10 };
            doc.Values["D1"] = 5;
            var result = AtomicPush.Run(doc, Change("D1", 50), Log.Null());
            Assert.False(result.Rebuilt);
            Assert.True(result.Restored);
            Assert.Equal(5, doc.Values["D1"]);
            Assert.Equal(2, doc.Rebuilds);
            Assert.Contains("R004", result.Diagnostic);
        }

        [Fact]
        public void Successful_rebuild_keeps_new_values()
        {
            var doc = new Doc();
            doc.Values["D1"] = 5;
            var result = AtomicPush.Run(doc, Change("D1", 7), Log.Null());
            Assert.True(result.Rebuilt);
            Assert.Null(result.Diagnostic);
            Assert.Equal(7, doc.Values["D1"]);
        }

        [Fact]
        public void Failed_restore_is_reported_in_diagnostic()
        {
            var doc = new Doc { FailAbove = 10, RefuseRestore = true };
            doc.Values["D1"] = 5;
            var result = AtomicPush.Run(doc, Change("D1", 50), Log.Null());
            Assert.False(result.Restored);
            Assert.Contains("restore also failed", result.Diagnostic);
        }
    }
}
