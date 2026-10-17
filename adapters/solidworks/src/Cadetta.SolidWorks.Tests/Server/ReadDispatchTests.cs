using System.IO;
using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Read;
using Cadetta.SolidWorks.Server;
using Cadetta.SolidWorks.Session;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Server
{
    public class ReadDispatchTests
    {
        private static RpcServer Server(INativeDocumentReader? reader)
        {
            var log = new Log(new StringWriter(), LogLevel.Trace);
            return new RpcServer(new FakeSwSession(), log, reader == null ? null : new ReadService(reader, null, log));
        }

        private static string Req(string files) =>
            "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"read\",\"params\":{\"source\":{\"files\":" + files + "}}}";

        private static NativeDocument Doc()
        {
            var d = new NativeDocument { Path = "a.sldprt" };
            d.Globals.Add(new NativeGlobal { Name = "W", Value = NativeQuantity.Of("12", "mm") });
            return d;
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Fact]
        public void Read_returns_observed_ir_for_one_file()
        {
            var reader = new FakeNativeDocumentReader().With(Doc());
            using var doc = JsonDocument.Parse(Server(reader).Handle(Req("[\"a.sldprt\"]"))!);
            var observed = doc.RootElement.GetProperty("result").GetProperty("observed");
            Assert.Equal("cadetta.ir/1", observed.GetProperty("schema").GetString());
            Assert.Equal("observed", observed.GetProperty("flavour").GetString());
            Assert.Equal(1, reader.Reads);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Fact]
        public void Read_of_a_missing_file_is_a_server_error()
        {
            using var doc = JsonDocument.Parse(Server(new FakeNativeDocumentReader()).Handle(Req("[\"nope.sldprt\"]"))!);
            Assert.Equal(-32000, doc.RootElement.GetProperty("error").GetProperty("code").GetInt32());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Theory]
        [InlineData("[]")]
        [InlineData("[\"a\",\"b\"]")]
        public void Read_needs_exactly_one_file(string files)
        {
            using var doc = JsonDocument.Parse(Server(new FakeNativeDocumentReader()).Handle(Req(files))!);
            Assert.Equal(-32602, doc.RootElement.GetProperty("error").GetProperty("code").GetInt32());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Fact]
        public void Read_without_a_service_is_method_not_found()
        {
            using var doc = JsonDocument.Parse(Server(null).Handle(Req("[\"a\"]"))!);
            Assert.Equal(-32601, doc.RootElement.GetProperty("error").GetProperty("code").GetInt32());
        }
    }
}
