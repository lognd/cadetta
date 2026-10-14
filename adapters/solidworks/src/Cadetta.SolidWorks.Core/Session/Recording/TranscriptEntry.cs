using System.IO;
using System.Text;
using System.Text.Json;

namespace Cadetta.SolidWorks.Session.Recording
{
    /// <summary>
    /// One recorded ISwSession call: its position, member name, arguments and
    /// result (or error), as the JSON line written to a transcript (solidworks.md 6).
    /// </summary>
    public sealed class TranscriptEntry
    {
        /// <summary>Creates an entry; <paramref name="argsJson"/> and <paramref name="resultJson"/> are raw JSON values.</summary>
        public TranscriptEntry(int seq, string call, string argsJson, string resultJson, string? error)
        {
            Seq = seq;
            Call = call;
            ArgsJson = argsJson;
            ResultJson = resultJson;
            Error = error;
        }

        /// <summary>Zero-based position of the call in the session.</summary>
        public int Seq { get; }

        /// <summary>The ISwSession member name.</summary>
        public string Call { get; }

        /// <summary>The arguments as a raw JSON array.</summary>
        public string ArgsJson { get; }

        /// <summary>The result as a raw JSON value ("null" when the call threw).</summary>
        public string ResultJson { get; }

        /// <summary>The exception message when the call threw, else null.</summary>
        public string? Error { get; }

        /// <summary>Renders the canonical one-line form: fixed key order, compact, no trailing newline.</summary>
        public string ToLine()
        {
            using var ms = new MemoryStream();
            using (var w = new Utf8JsonWriter(ms))
            {
                w.WriteStartObject();
                w.WriteNumber("seq", Seq);
                w.WriteString("call", Call);
                w.WritePropertyName("args");
                w.WriteRawValue(ArgsJson);
                w.WritePropertyName("result");
                w.WriteRawValue(ResultJson);
                if (Error != null)
                {
                    w.WriteString("error", Error);
                }
                w.WriteEndObject();
            }
            return Encoding.UTF8.GetString(ms.ToArray());
        }

        /// <summary>Parses one transcript line; throws <see cref="JsonException"/> on malformed input.</summary>
        public static TranscriptEntry Parse(string line)
        {
            using var doc = JsonDocument.Parse(line);
            var root = doc.RootElement;
            var error = root.TryGetProperty("error", out var e) ? e.GetString() : null;
            return new TranscriptEntry(
                root.GetProperty("seq").GetInt32(),
                root.GetProperty("call").GetString() ?? throw new JsonException("call is null"),
                root.GetProperty("args").GetRawText(),
                root.GetProperty("result").GetRawText(),
                error);
        }
    }
}
