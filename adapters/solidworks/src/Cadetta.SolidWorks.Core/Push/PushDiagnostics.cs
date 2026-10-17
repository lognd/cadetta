using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Cadetta.SolidWorks.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>Builds `cadetta.diag/1` records (lints.md 1.1) for the diagnostics a push reports.</summary>
    public static class PushDiagnostics
    {
        /// <summary>Builds one record; <paramref name="path"/> is the canonical path of the subject, if any.</summary>
        public static JsonElement Make(string code, string severity, string message, string? path)
        {
            var rec = new Record { Code = code, Severity = severity, Message = message, Path = path };
            return JsonSerializer.SerializeToElement(rec);
        }

        private sealed class Record
        {
            [JsonPropertyName("schema")]
            public string Schema { get; set; } = "cadetta.diag/1";

            [JsonPropertyName("code")]
            public string Code { get; set; } = "";

            [JsonPropertyName("severity")]
            public string Severity { get; set; } = "error";

            [JsonPropertyName("message")]
            public string Message { get; set; } = "";

            [JsonPropertyName("path")]
            public string? Path { get; set; }

            [JsonPropertyName("notes")]
            public List<string> Notes { get; set; } = new List<string>();

            [JsonPropertyName("fixes")]
            public List<string> Fixes { get; set; } = new List<string>();
        }
    }
}
