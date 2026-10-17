using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;
using Cadetta.SolidWorks.Save;
using Cadetta.SolidWorks.Server;
using Cadetta.SolidWorks.Session;
using Cadetta.SolidWorks.Tests.Push;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Server
{
    // frob:ticket 01M5496CQE8FPX6F7PD9Y6EMX0
    public class PushDispatchTests
    {
        private sealed class Target : ISaveTarget
        {
            public string Path { get; set; } = "";
            public bool Modified { get; set; }
            public bool RebuildFailed { get; set; }
            public int Saves;
            public string? Save() { Saves++; return null; }
        }

        private sealed class Handle : IPushHandle
        {
            public FakePushDocument Doc = new FakePushDocument();
            public Target Tgt = new Target();
            public bool Disposed;
            public IPushDocument Document => Doc;
            public ISaveTarget Target => Tgt;
            public void Settle(bool modified, bool rebuildFailed) { Tgt.Modified = modified; Tgt.RebuildFailed = rebuildFailed; }
            public void Dispose() { Disposed = true; }
        }

        private sealed class Host : IPushHost
        {
            public Handle? H = new Handle { Doc = { AdapterModified = true } };
            public IPushHandle Open(string file)
            {
                if (H == null) throw new InvalidOperationException("OpenDoc6 failed for " + file);
                H.Doc.Path = file;
                H.Tgt.Path = file;
                return H;
            }
        }

        private static RpcServer Server(Host? host) =>
            new RpcServer(new FakeSwSession(), new Log(new StringWriter(), LogLevel.Trace), null, host);

        private static string Req(string file, string paramsJson = "[{\"locator\":\"WIDTH\",\"mode\":\"value\",\"path\":\"p.w\",\"value\":\"6\"}]") =>
            "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"push\",\"params\":{\"file\":" + JsonSerializer.Serialize(file) + ",\"params\":" + paramsJson + ",\"session\":\"s\"}}";

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Fact]
        public void Push_writes_reports_statuses_and_saves()
        {
            var file = System.IO.Path.GetTempFileName();
            try
            {
                var host = new Host();
                using var doc = JsonDocument.Parse(Server(host).Handle(Req(file))!);
                var statuses = doc.RootElement.GetProperty("result").GetProperty("statuses");
                Assert.Equal("pushed", statuses[0].GetProperty("status").GetString());
                Assert.Equal(6, host.H!.Doc.Globals["WIDTH"]);
                Assert.Equal(1, host.H.Tgt.Saves);
                Assert.True(host.H.Disposed);
            }
            finally
            {
                File.Delete(file);
            }
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Fact]
        public void Push_of_an_unopenable_file_is_a_server_error()
        {
            using var doc = JsonDocument.Parse(Server(new Host { H = null }).Handle(Req("nope.sldprt"))!);
            Assert.Equal(-32000, doc.RootElement.GetProperty("error").GetProperty("code").GetInt32());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Fact]
        public void Push_without_a_file_is_invalid_params()
        {
            using var doc = JsonDocument.Parse(Server(new Host()).Handle(Req(""))!);
            Assert.Equal(-32602, doc.RootElement.GetProperty("error").GetProperty("code").GetInt32());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Fact]
        public void Push_without_a_host_is_method_not_found()
        {
            using var doc = JsonDocument.Parse(Server(null).Handle(Req("a.sldprt"))!);
            Assert.Equal(-32601, doc.RootElement.GetProperty("error").GetProperty("code").GetInt32());
        }
    }
}
