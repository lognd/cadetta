using System.IO;
using System.Text.Json;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Server;
using Cadetta.SolidWorks.Session;
using Xunit;

namespace Cadetta.SolidWorks.Transcripts.Tests
{
    // frob:ticket ~NEHY9X7
    /// <summary>The drive session methods `open`, `save` and `close` answer instead of MethodNotFound (found by the live smoke, R009).</summary>
    public class DriveSessionTests
    {
        private static RpcServer Server() => new RpcServer(new FakeSwSession(), new Log(new StringWriter(), LogLevel.Trace));

        // frob:tests Cadetta.SolidWorks.Server.RpcServer.Handle
        [Fact]
        public void OpenSaveCloseAreServed()
        {
            var s = Server();
            var open = JsonDocument.Parse(s.Handle("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"open\",\"params\":{\"files\":[\"a.sldprt\"]}}")!);
            var handle = open.RootElement.GetProperty("result").GetProperty("session").GetProperty("handle").GetString();
            Assert.False(string.IsNullOrEmpty(handle));
            Assert.Equal("a.sldprt", open.RootElement.GetProperty("result").GetProperty("session").GetProperty("files")[0].GetProperty("path").GetString());
            var save = JsonDocument.Parse(s.Handle("{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"save\",\"params\":{\"session\":\"" + handle + "\"}}")!);
            Assert.True(save.RootElement.TryGetProperty("result", out _));
            var close = JsonDocument.Parse(s.Handle("{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"close\",\"params\":{\"session\":\"" + handle + "\"}}")!);
            Assert.True(close.RootElement.TryGetProperty("result", out _));
        }
    }
}
