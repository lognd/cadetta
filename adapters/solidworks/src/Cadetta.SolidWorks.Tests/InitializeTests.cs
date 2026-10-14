using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Server;
using Cadetta.SolidWorks.Session;
using Xunit;

namespace Cadetta.SolidWorks.Tests
{
    public class InitializeTests
    {
        private const string Init =
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":" +
            "{\"protocol\":\"cadetta.adapter/1\",\"client\":{\"name\":\"cadetta\",\"version\":\"0.0.0\"}}}";

        private static (int code, List<string> stdout, string stderr) Serve(params string[] lines)
        {
            var err = new StringWriter();
            var server = new RpcServer(new FakeSwSession(), new Log(err, LogLevel.Trace));
            var input = new StringReader(string.Join("\n", lines) + "\n");
            var output = new StringWriter { NewLine = "\n" };
            var code = server.Run(input, output);
            var outLines = output.ToString().Split('\n').Where(l => l.Length > 0).ToList();
            return (code, outLines, err.ToString());
        }

        [Fact]
        public void Initialize_reports_identity_capabilities_and_features()
        {
            var (_, stdout, _) = Serve(Init);
            using var doc = JsonDocument.Parse(Assert.Single(stdout));
            var r = doc.RootElement.GetProperty("result").Deserialize<InitializeResult>()!;
            Assert.Equal("solidworks", r.Name);
            Assert.Equal("0.0.0", r.Version);
            Assert.Equal(new[] { "read", "drive" }, r.Capabilities);
            Assert.Equal(
                new[] { "frames", "instances", "create_frames", "tolerances", "rename_globals" },
                r.Features);
            Assert.Equal("mm", r.NativeUnits);
            Assert.Equal(6u, r.Precision);
            Assert.Equal(new[] { "windows" }, r.Platforms);
        }

        [Fact]
        public void Stdout_carries_protocol_only_and_logs_go_to_stderr()
        {
            var (code, stdout, stderr) = Serve(
                Init, "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"shutdown\"}");
            Assert.Equal(0, code);
            Assert.Equal(2, stdout.Count);
            foreach (var line in stdout)
            {
                using var doc = JsonDocument.Parse(line);
                Assert.Equal("2.0", doc.RootElement.GetProperty("jsonrpc").GetString());
            }
            Assert.Contains("INFO cadetta_solidworks: initialize from cadetta", stderr);
        }

        [Fact]
        public void Wrong_protocol_is_invalid_params()
        {
            var (_, stdout, _) = Serve(
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":" +
                "{\"protocol\":\"other/9\",\"client\":{\"name\":\"x\",\"version\":\"1\"}}}");
            using var doc = JsonDocument.Parse(Assert.Single(stdout));
            Assert.Equal(-32602, doc.RootElement.GetProperty("error").GetProperty("code").GetInt32());
        }

        [Fact]
        public void Unimplemented_method_and_garbage_are_errors_not_crashes()
        {
            var (code, stdout, _) = Serve(
                "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"read\",\"params\":{}}",
                "not json");
            Assert.Equal(0, code);
            Assert.Equal(2, stdout.Count);
            using var a = JsonDocument.Parse(stdout[0]);
            Assert.Equal(-32601, a.RootElement.GetProperty("error").GetProperty("code").GetInt32());
            Assert.Equal(7, a.RootElement.GetProperty("id").GetInt32());
            using var b = JsonDocument.Parse(stdout[1]);
            Assert.Equal(-32700, b.RootElement.GetProperty("error").GetProperty("code").GetInt32());
        }

        [Fact]
        public void Notifications_get_no_reply()
        {
            var (_, stdout, _) = Serve("{\"jsonrpc\":\"2.0\",\"method\":\"initialized\"}");
            Assert.Empty(stdout);
        }

        [Fact]
        public void Log_filter_parses_bare_levels_and_targeted_directives()
        {
            Assert.Equal(LogLevel.Warn, Log.ParseFilter(null));
            Assert.Equal(LogLevel.Debug, Log.ParseFilter("debug"));
            Assert.Equal(LogLevel.Trace, Log.ParseFilter("warn,cadetta_solidworks=trace"));
            Assert.Equal(LogLevel.Warn, Log.ParseFilter("other=trace"));
        }
    }
}
