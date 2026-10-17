using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Session.Recording;
using Xunit;

namespace Cadetta.SolidWorks.Transcripts.Tests
{
    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>Replays recorded read, push and regen exchanges byte for byte and checks the adapters.md 4 contract rules against them.</summary>
    public class ContractTests
    {
        public static IEnumerable<object[]> Names() => Scenarios.All.Select(s => new object[] { s.Name });

        private static Scenario Get(string name) => Scenarios.All.Single(s => s.Name == name);

        private static string FixtureDir()
        {
            var dir = new DirectoryInfo(AppContext.BaseDirectory);
            while (dir != null && !File.Exists(Path.Combine(dir.FullName, "Cadetta.SolidWorks.Transcripts.Tests.csproj")))
            {
                dir = dir.Parent;
            }
            return dir?.FullName ?? throw new InvalidOperationException("transcripts directory not found above " + AppContext.BaseDirectory);
        }

        private static string FixturePath(string name) => Path.Combine(FixtureDir(), name + ".jsonl");

        private static string NewFile()
        {
            var file = Path.GetTempFileName();
            File.WriteAllBytes(file, new byte[] { 99, 100, 116, 10 });
            return file;
        }

        private static string Replay(string name, out Tape tape, Scenario? with = null)
        {
            var file = NewFile();
            try
            {
                tape = Tape.Load(FixturePath(name), file);
                return Scenarios.Run(with ?? Get(name), tape);
            }
            finally
            {
                File.Delete(file);
            }
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Fact]
        public void Record_fixtures_when_CADETTA_RECORD_TRANSCRIPTS_is_set()
        {
            var outDir = Environment.GetEnvironmentVariable("CADETTA_RECORD_TRANSCRIPTS");
            if (string.IsNullOrEmpty(outDir)) return;
            Directory.CreateDirectory(outDir);
            foreach (var s in Scenarios.All)
            {
                var file = NewFile();
                try
                {
                    var tape = Tape.Record(file);
                    Scenarios.Run(s, tape);
                    File.WriteAllText(Path.Combine(outDir, s.Name + ".jsonl"), tape.ToText());
                }
                finally
                {
                    File.Delete(file);
                }
            }
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Theory]
        [MemberData(nameof(Names))]
        public void Replay_is_byte_exact_and_consumes_every_recorded_call(string name)
        {
            Replay(name, out var tape);
            Assert.True(tape.Exhausted);
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Theory]
        [MemberData(nameof(Names))]
        public void Rule2_same_inputs_give_same_bytes(string name)
        {
            var a = Replay(name, out var ta);
            var b = Replay(name, out var tb);
            Assert.Equal(ta.Normalize(a), tb.Normalize(b));
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Fact]
        public void A_changed_request_diverges_from_the_transcript()
        {
            var changed = new Scenario("push-ok", Get("push-ok").Request.Replace("\"12\"", "\"13\""), _ => { });
            Assert.ThrowsAny<Exception>(() => Replay("push-ok", out _, changed));
        }

        private static JsonElement Result(string response)
        {
            using var doc = JsonDocument.Parse(response);
            return doc.RootElement.GetProperty("result").Clone();
        }

        private static List<string> Codes(JsonElement result) =>
            result.GetProperty("diagnostics").EnumerateArray().Select(d => d.GetProperty("code").GetString()!).ToList();

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Fact]
        public void Rule3_an_unrepresentable_value_is_Q004_and_never_written()
        {
            var response = Replay("push-q004", out var tape);
            Assert.Contains("Q004", Codes(Result(response)));
            Assert.DoesNotContain(tape.Entries, e => e.Call == "Doc.SetGlobal");
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Fact]
        public void Rule4_a_driven_dimension_reads_back_inexact_with_a_reason()
        {
            var response = Replay("read-observed", out _);
            var text = Result(response).GetRawText();
            Assert.Contains("\"exact\":false", text.Replace(" ", ""));
            Assert.Contains("driven", text);
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Theory]
        [MemberData(nameof(Names))]
        public void Rule5_no_exchange_touches_cdt_source(string name)
        {
            Replay(name, out var tape);
            Assert.DoesNotContain(tape.Entries, e => e.ArgsJson.Contains(".cdt") || e.ResultJson.Contains(".cdt"));
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Theory]
        [InlineData("push-ok", 3, new string[0])]
        [InlineData("push-q004", 1, new[] { "Q004" })]
        [InlineData("push-r004-rebuild", 1, new[] { "R004" })]
        [InlineData("push-r006-equation", 1, new[] { "R006" })]
        [InlineData("push-r009-readonly", 1, new[] { "R009" })]
        [InlineData("push-r010-driven", 1, new[] { "R010" })]
        public void Rule6_every_locator_gets_a_status_and_every_refusal_a_coded_diagnostic(string name, int locators, string[] codes)
        {
            var result = Result(Replay(name, out _));
            Assert.Equal(locators, result.GetProperty("statuses").GetArrayLength());
            foreach (var c in codes) Assert.Contains(c, Codes(result));
            if (codes.Length == 0) Assert.Empty(Codes(result));
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Theory]
        [InlineData("regen-clean", "ok", null)]
        [InlineData("regen-r004", "failed", "R004")]
        [InlineData("regen-r009-dirty", "failed", "R009")]
        public void Regen_transcripts_report_target_status_and_codes(string name, string status, string? code)
        {
            var result = Result(Replay(name, out _));
            Assert.Equal(status, result.GetProperty("targets")[0].GetProperty("status").GetString());
            if (code == null) Assert.Empty(Codes(result)); else Assert.Contains(code, Codes(result));
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Fact]
        public void A_missing_file_read_is_a_structured_server_error()
        {
            using var doc = JsonDocument.Parse(Replay("read-missing", out _));
            Assert.Equal(-32000, doc.RootElement.GetProperty("error").GetProperty("code").GetInt32());
        }

        // frob:tests adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/Scenarios.cs::Scenarios.Run kind=unit
        [Fact]
        public void The_original_product_version_transcript_still_replays()
        {
            var path = Path.Combine(FixtureDir(), "product-version.jsonl");
            var log = new Cadetta.SolidWorks.Logging.Log(new StringWriter(), Cadetta.SolidWorks.Logging.LogLevel.Trace);
            using var reader = new StreamReader(path);
            var session = ReplaySwSession.FromReader(reader, log);
            Assert.Equal("32.1.0", session.ProductVersion());
            Assert.Equal("32.1.0", session.ProductVersion());
            Assert.True(session.Exhausted);
        }
    }
}
