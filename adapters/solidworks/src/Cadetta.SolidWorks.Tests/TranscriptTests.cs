using System.IO;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Session;
using Cadetta.SolidWorks.Session.Recording;
using Xunit;

namespace Cadetta.SolidWorks.Tests
{
    public class TranscriptTests
    {
        private static string Fixture(string name)
        {
            var dir = new DirectoryInfo(System.AppContext.BaseDirectory);
            while (dir != null && !File.Exists(Path.Combine(dir.FullName, "Cadetta.SolidWorks.sln")))
            {
                dir = dir.Parent;
            }
            return Path.Combine(dir!.FullName, "tests", "Cadetta.SolidWorks.Tests", "Transcripts", name);
        }

        [Fact]
        public void Recorded_calls_replay_to_the_same_results()
        {
            var sink = new StringWriter { NewLine = "\n" };
            var rec = new RecordingSwSession(new FakeSwSession(), sink, Log.Null());
            Assert.Equal("fake", rec.ProductVersion());
            var replay = ReplaySwSession.FromReader(new StringReader(sink.ToString()), Log.Null());
            Assert.Equal("fake", replay.ProductVersion());
            Assert.True(replay.Exhausted);
        }

        [Fact]
        public void Replay_then_rerecord_is_byte_exact()
        {
            var original = File.ReadAllText(Fixture("product-version.jsonl")).Replace("\r\n", "\n");
            var replay = ReplaySwSession.FromReader(new StringReader(original), Log.Null());
            var sink = new StringWriter { NewLine = "\n" };
            var rec = new RecordingSwSession(replay, sink, Log.Null());
            Assert.Equal("32.1.0", rec.ProductVersion());
            Assert.Equal("32.1.0", rec.ProductVersion());
            Assert.Equal(original, sink.ToString());
        }

        [Fact]
        public void Extra_call_beyond_the_transcript_is_a_mismatch()
        {
            var replay = ReplaySwSession.FromReader(
                new StringReader("{\"seq\":0,\"call\":\"ProductVersion\",\"args\":[],\"result\":\"x\"}\n"), Log.Null());
            Assert.Equal("x", replay.ProductVersion());
            Assert.Throws<TranscriptMismatchException>(() => replay.ProductVersion());
        }

        [Fact]
        public void Wrong_call_name_is_a_mismatch()
        {
            var replay = ReplaySwSession.FromReader(
                new StringReader("{\"seq\":0,\"call\":\"Other\",\"args\":[],\"result\":\"x\"}\n"), Log.Null());
            Assert.Throws<TranscriptMismatchException>(() => replay.ProductVersion());
        }
    }
}
