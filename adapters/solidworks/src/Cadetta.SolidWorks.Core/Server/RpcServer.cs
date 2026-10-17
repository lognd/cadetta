using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Push;
using Cadetta.SolidWorks.Read;
using Cadetta.SolidWorks.Regen;
using Cadetta.SolidWorks.Save;
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
        private const int ServerError = -32000;

        private static readonly JsonSerializerOptions Options = new JsonSerializerOptions
        {
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        };

        private readonly ISwSession _session;
        private readonly Log _log;
        private readonly ReadService? _read;
        private readonly IPushHost? _pushHost;
        private readonly RegenService? _regen;
        private readonly PushService _push;
        private readonly SaveService _save;
        private bool _stop;

        /// <summary>Creates a server over a session; `read` is answered by <paramref name="read"/> and `push` by <paramref name="pushHost"/> when given and `regen` by <paramref name="regenHost"/> when given (frob:ticket 01M548Y09E8W1NVTJSSS0GDS2T, 01M5496CQE8FPX6F7PD9Y6EMX0, 01M4SV2TVM50WJRZ42ZHXVD96N).</summary>
        public RpcServer(ISwSession session, Log log, ReadService? read = null, IPushHost? pushHost = null, IRegenHost? regenHost = null)
        {
            _session = session;
            _log = log;
            _read = read;
            _pushHost = pushHost;
            _regen = regenHost == null ? null : new RegenService(regenHost, log);
            _push = new PushService(log);
            _save = new SaveService(new DiskFileReader(), log);
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
                case "read" when _read != null:
                    return Read(id, ps);
                case "push" when _pushHost != null:
                    return Push(id, ps);
                case "regen" when _regen != null:
                    return Regen(id, ps);
                case "shutdown":
                    _stop = true;
                    return Result(id, "null");
                default:
                    _log.Warn("method not implemented: " + method);
                    return Error(id, MethodNotFound, "method not implemented: " + method);
            }
        }

        /// <summary>Answers `read` for a single file through the read service (frob:ticket 01M548Y09E8W1NVTJSSS0GDS2T).</summary>
        private string Read(JsonElement id, JsonElement ps)
        {
            ReadParams? p;
            try
            {
                p = ps.ValueKind == JsonValueKind.Undefined ? null : ps.Deserialize<ReadParams>(Options);
            }
            catch (JsonException e)
            {
                _log.Warn("bad read params: " + e.Message);
                return Error(id, InvalidParams, "bad read params: " + e.Message);
            }
            var files = p?.Source?.Files;
            if (files == null || files.Count != 1)
            {
                _log.Warn("read needs source.files with exactly one file");
                return Error(id, InvalidParams, "read needs source.files with exactly one file");
            }
            try
            {
                var result = _read!.Read(files[0]);
                _log.Info("read " + files[0] + " done");
                return Result(id, JsonSerializer.Serialize(result, Options));
            }
            catch (Exception e) when (e is IOException || e is InvalidOperationException || e is System.Runtime.InteropServices.COMException)
            {
                _log.Error("read " + files[0] + " failed: " + e.Message);
                return Error(id, ServerError, "read failed: " + e.Message);
            }
        }

        /// <summary>Answers `push`: opens the file, pushes every locator, then saves per the default mode (frob:ticket 01M5496CQE8FPX6F7PD9Y6EMX0).</summary>
        private string Push(JsonElement id, JsonElement ps)
        {
            PushParams? p;
            try
            {
                p = ps.ValueKind == JsonValueKind.Undefined ? null : ps.Deserialize<PushParams>(Options);
            }
            catch (JsonException e)
            {
                _log.Warn("bad push params: " + e.Message);
                return Error(id, InvalidParams, "bad push params: " + e.Message);
            }
            if (p == null || p.File.Length == 0)
            {
                _log.Warn("push needs a file");
                return Error(id, InvalidParams, "push needs a file");
            }
            try
            {
                using (var handle = _pushHost!.Open(p.File))
                {
                    var options = new PushOptions { Precision = (int)AdapterInfo.Precision };
                    var outcome = _push.Push(handle.Document, p.Params, options);
                    handle.Settle(handle.Document.AdapterModified, outcome.RebuildFailed);
                    outcome.Save = _save.Run(options.Save, outcome.AnyFailed, new[] { handle.Target });
                    foreach (var d in outcome.Save.Diagnostics)
                    {
                        outcome.Diagnostics.Add(PushDiagnostics.Make(PushService.R009, "warning", d, null));
                    }
                    _log.Info("push " + p.File + " done: " + outcome.Statuses.Count + " statuses");
                    return Result(id, JsonSerializer.Serialize(outcome.ToResult(), Options));
                }
            }
            catch (Exception e) when (e is IOException || e is InvalidOperationException || e is System.Runtime.InteropServices.COMException)
            {
                _log.Error("push " + p.File + " failed: " + e.Message);
                return Error(id, ServerError, "push failed: " + e.Message);
            }
        }

        /// <summary>Answers `regen`: rebuilds each target file and reports R004 per failed feature (frob:ticket 01M4SV2TVM50WJRZ42ZHXVD96N).</summary>
        private string Regen(JsonElement id, JsonElement ps)
        {
            RegenParams? p;
            try
            {
                p = ps.ValueKind == JsonValueKind.Undefined ? null : ps.Deserialize<RegenParams>(Options);
            }
            catch (JsonException e)
            {
                _log.Warn("bad regen params: " + e.Message);
                return Error(id, InvalidParams, "bad regen params: " + e.Message);
            }
            if (p == null || p.Targets.Count == 0)
            {
                _log.Warn("regen needs at least one target file");
                return Error(id, InvalidParams, "regen needs at least one target file");
            }
            try
            {
                var result = _regen!.Regen(p.Targets);
                return Result(id, JsonSerializer.Serialize(result, Options));
            }
            catch (Exception e) when (e is IOException || e is InvalidOperationException || e is System.Runtime.InteropServices.COMException)
            {
                _log.Error("regen failed: " + e.Message);
                return Error(id, ServerError, "regen failed: " + e.Message);
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
