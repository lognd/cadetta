using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Session;

namespace Cadetta.SolidWorks.Server
{
    /// <summary>
    /// Newline-delimited JSON-RPC 2.0 server (adapters.md 5). It writes protocol
    /// to the output writer only; logs go to the injected <see cref="Log"/>.
    /// </summary>
    public sealed class RpcServer
    {
        private const int ParseError = -32700;
        private const int InvalidRequest = -32600;
        private const int MethodNotFound = -32601;
        private const int InvalidParams = -32602;

        private static readonly JsonSerializerOptions Options = new JsonSerializerOptions
        {
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        };

        private readonly ISwSession _session;
        private readonly Log _log;
        private bool _stop;

        /// <summary>Creates a server over a session.</summary>
        public RpcServer(ISwSession session, Log log)
        {
            _session = session;
            _log = log;
        }

        /// <summary>The adapter's initialize result: read and drive plus the five features.</summary>
        public static InitializeResult Describe() => new InitializeResult
        {
            Name = AdapterInfo.Name,
            Version = AdapterInfo.Version,
            Capabilities = new List<string> { CapabilityName.Read, CapabilityName.Drive },
            Features = new List<string>
            {
                Feature.Frames,
                Feature.Instances,
                Feature.CreateFrames,
                Feature.Tolerances,
                Feature.RenameGlobals,
            },
            NativeUnits = AdapterInfo.NativeUnits,
            Precision = AdapterInfo.Precision,
            Platforms = new List<string> { "windows" },
        };

        /// <summary>Serves until shutdown or end of input; returns the process exit code.</summary>
        public int Run(TextReader input, TextWriter output)
        {
            _log.Info("serving the adapter protocol on stdio (SOLIDWORKS " + _session.ProductVersion() + ")");
            string? line;
            while (!_stop && (line = input.ReadLine()) != null)
            {
                if (line.Trim().Length == 0)
                {
                    continue;
                }
                var reply = Handle(line);
                if (reply != null)
                {
                    output.Write(reply + "\n");
                    output.Flush();
                }
            }
            _log.Info(_stop ? "shutdown requested" : "input closed");
            return 0;
        }

        /// <summary>Handles one request line; returns the response line, or null for a notification.</summary>
        public string? Handle(string line)
        {
            JsonDocument doc;
            try
            {
                doc = JsonDocument.Parse(line);
            }
            catch (JsonException e)
            {
                _log.Warn("unparsable message: " + e.Message);
                return Error(null, ParseError, "parse error");
            }
            using (doc)
            {
                var root = doc.RootElement;
                if (root.ValueKind != JsonValueKind.Object
                    || !root.TryGetProperty("method", out var methodEl)
                    || methodEl.ValueKind != JsonValueKind.String)
                {
                    _log.Warn("message is not a request object");
                    return Error(null, InvalidRequest, "invalid request");
                }
                JsonElement? id = root.TryGetProperty("id", out var idEl) ? idEl.Clone() : (JsonElement?)null;
                var method = methodEl.GetString()!;
                _log.Debug("request " + method);
                if (id == null)
                {
                    _log.Debug("notification " + method + " ignored");
                    return null;
                }
                root.TryGetProperty("params", out var ps);
                return Dispatch(id.Value, method, ps);
            }
        }

        private string Dispatch(JsonElement id, string method, JsonElement ps)
        {
            switch (method)
            {
                case "initialize":
                    return Initialize(id, ps);
                case "shutdown":
                    _stop = true;
                    return Result(id, "null");
                default:
                    _log.Warn("method not implemented: " + method);
                    return Error(id, MethodNotFound, "method not implemented: " + method);
            }
        }

        private string Initialize(JsonElement id, JsonElement ps)
        {
            InitializeParams? p;
            try
            {
                p = ps.ValueKind == JsonValueKind.Undefined ? null : ps.Deserialize<InitializeParams>(Options);
            }
            catch (JsonException e)
            {
                _log.Warn("bad initialize params: " + e.Message);
                return Error(id, InvalidParams, "bad initialize params: " + e.Message);
            }
            if (p == null || p.Protocol != ProtocolInfo.Protocol)
            {
                _log.Error("unsupported protocol " + (p?.Protocol ?? "<none>"));
                return Error(id, InvalidParams,
                    "unsupported protocol; this adapter speaks " + ProtocolInfo.Protocol);
            }
            _log.Info("initialize from " + p.Client?.Name + " " + p.Client?.Version);
            return Result(id, JsonSerializer.Serialize(Describe(), Options));
        }

        private static string Result(JsonElement id, string rawResult)
        {
            return Envelope(id, w =>
            {
                w.WritePropertyName("result");
                w.WriteRawValue(rawResult);
            });
        }

        private static string Error(JsonElement? id, int code, string message)
        {
            return Envelope(id, w =>
            {
                w.WriteStartObject("error");
                w.WriteNumber("code", code);
                w.WriteString("message", message);
                w.WriteEndObject();
            });
        }

        private static string Envelope(JsonElement? id, Action<Utf8JsonWriter> body)
        {
            using (var ms = new MemoryStream())
            {
                using (var w = new Utf8JsonWriter(ms))
                {
                    w.WriteStartObject();
                    w.WriteString("jsonrpc", "2.0");
                    w.WritePropertyName("id");
                    if (id.HasValue)
                    {
                        id.Value.WriteTo(w);
                    }
                    else
                    {
                        w.WriteNullValue();
                    }
                    body(w);
                    w.WriteEndObject();
                }
                return Encoding.UTF8.GetString(ms.ToArray());
            }
        }
    }
}
