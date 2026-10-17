using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Regen;
using Cadetta.SolidWorks.Server;
using Cadetta.SolidWorks.Session;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Regen
{
    // frob:ticket 01M4SV2TVM50WJRZ42ZHXVD96N
    public class RegenServiceTests
    {
        private sealed class Doc : IRegenDocument
        {
            public string Path { get; set; } = "";
            public bool ReadOnly { get; set; }
            public bool UserDirty { get; set; }
            public List<RebuildError> Errors = new List<RebuildError>();
            public int Rebuilds;
            public IReadOnlyList<RebuildError> Rebuild() { Rebuilds++; return Errors; }
        }

        private sealed class Handle : IRegenHandle
        {
            public Doc Doc = new Doc();
            public bool Disposed;
            public IRegenDocument Document => Doc;
            public void Dispose() { Disposed = true; }
        }

        private sealed class Host : IRegenHost
        {
            public Handle H = new Handle();
            public bool Unopenable;
            public IRegenHandle Open(string file)
            {
                if (Unopenable) throw new InvalidOperationException("OpenDoc6 failed for " + file);
                H.Doc.Path = file;
                return H;
            }
        }

        private static Log NewLog() => new Log(new StringWriter(), LogLevel.Trace);

        private static string Code(System.Text.Json.JsonElement d) => d.GetProperty("code").GetString()!;

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Regen/RegenService.cs::RegenService.Regen kind=unit
        [Fact]
        public void Clean_rebuild_is_ok_without_diagnostics()
        {
            var host = new Host();
            var r = new RegenService(host, NewLog()).Regen(new[] { "a.sldprt" });
            Assert.Equal("ok", r.Targets[0].Status);
            Assert.Equal("a.sldprt", r.Targets[0].Target);
            Assert.Empty(r.Diagnostics);
            Assert.Equal(1, host.H.Doc.Rebuilds);
            Assert.True(host.H.Disposed);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Regen/RegenService.cs::RegenService.Regen kind=unit
        [Fact]
        public void Rebuild_error_is_R004_naming_the_feature()
        {
            var host = new Host();
            host.H.Doc.Errors.Add(new RebuildError { Feature = "Boss-Extrude1", Code = 4, Message = "feature error code 4" });
            var r = new RegenService(host, NewLog()).Regen(new[] { "a.sldprt" });
            Assert.Equal("failed", r.Targets[0].Status);
            var d = Assert.Single(r.Diagnostics);
            Assert.Equal("R004", Code(d));
            Assert.Contains("Boss-Extrude1", d.GetProperty("message").GetString());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Regen/RegenService.cs::RegenService.Regen kind=unit
        [Fact]
        public void Unsaved_user_changes_refuse_the_regen_with_R009()
        {
            var host = new Host();
            host.H.Doc.UserDirty = true;
            var r = new RegenService(host, NewLog()).Regen(new[] { "a.sldprt" });
            Assert.Equal("failed", r.Targets[0].Status);
            Assert.Equal("R009", Code(Assert.Single(r.Diagnostics)));
            Assert.Equal(0, host.H.Doc.Rebuilds);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Regen/RegenService.cs::RegenService.Regen kind=unit
        [Fact]
        public void Read_only_document_is_refused_with_R009()
        {
            var host = new Host();
            host.H.Doc.ReadOnly = true;
            var r = new RegenService(host, NewLog()).Regen(new[] { "a.sldprt" });
            Assert.Equal("R009", Code(Assert.Single(r.Diagnostics)));
            Assert.Equal(0, host.H.Doc.Rebuilds);
        }

        private static RpcServer Server(Host host) =>
            new RpcServer(new FakeSwSession(), NewLog(), null, null, host);

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Fact]
        public void Rpc_regen_returns_target_statuses_and_diagnostics()
        {
            var host = new Host();
            host.H.Doc.Errors.Add(new RebuildError { Feature = "Fillet1", Message = "bad" });
            var req = "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"regen\",\"params\":{\"session\":\"s\",\"targets\":[\"a.sldprt\"]}}";
            using var doc = JsonDocument.Parse(Server(host).Handle(req)!);
            var result = doc.RootElement.GetProperty("result");
            Assert.Equal("failed", result.GetProperty("targets")[0].GetProperty("status").GetString());
            Assert.Equal("R004", result.GetProperty("diagnostics")[0].GetProperty("code").GetString());
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs::RpcServer.Handle kind=unit
        [Fact]
        public void Rpc_regen_without_targets_is_invalid_params_and_unopenable_is_server_error()
        {
            var host = new Host();
            var none = "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"regen\",\"params\":{\"session\":\"s\",\"targets\":[]}}";
            using (var d1 = JsonDocument.Parse(Server(host).Handle(none)!))
            {
                Assert.Equal(-32602, d1.RootElement.GetProperty("error").GetProperty("code").GetInt32());
            }
            host.Unopenable = true;
            var one = "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"regen\",\"params\":{\"session\":\"s\",\"targets\":[\"a.sldprt\"]}}";
            using (var d2 = JsonDocument.Parse(Server(host).Handle(one)!))
            {
                Assert.Equal(-32000, d2.RootElement.GetProperty("error").GetProperty("code").GetInt32());
            }
        }
    }
}
