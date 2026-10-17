using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using Cadetta.SolidWorks.Session.Recording;

namespace Cadetta.SolidWorks.Transcripts.Tests
{
    // frob:ticket 01M4X91P37CRMWF43MHK53RXEG
    /// <summary>An ordered call log: records native document calls while a scenario runs against fakes, or replays them strictly from a fixture.</summary>
    internal sealed class Tape
    {
        private const string FileToken = "{file}";
        private static readonly JsonSerializerOptions Opts = new JsonSerializerOptions
        {
            PreferredObjectCreationHandling = System.Text.Json.Serialization.JsonObjectCreationHandling.Populate,
        };

        private readonly List<TranscriptEntry> _entries;
        private readonly bool _recording;
        private readonly string _escapedFile;
        private int _next;

        private Tape(List<TranscriptEntry> entries, bool recording, string file)
        {
            _entries = entries;
            _recording = recording;
            var quoted = JsonSerializer.Serialize(file);
            _escapedFile = quoted.Substring(1, quoted.Length - 2);
        }

        /// <summary>Starts an empty recording; the volatile <paramref name="file"/> path is stored as a token.</summary>
        public static Tape Record(string file) => new Tape(new List<TranscriptEntry>(), true, file);

        /// <summary>Loads a fixture for strict replay; the token is mapped back to <paramref name="file"/>.</summary>
        public static Tape Load(string fixture, string file)
        {
            var entries = new List<TranscriptEntry>();
            foreach (var line in File.ReadAllLines(fixture))
            {
                if (line.Length > 0) entries.Add(TranscriptEntry.Parse(line));
            }
            return new Tape(entries, false, file);
        }

        /// <summary>True when recording from live fakes rather than replaying.</summary>
        public bool Recording => _recording;

        /// <summary>The entries so far, in order.</summary>
        public IReadOnlyList<TranscriptEntry> Entries => _entries;

        /// <summary>True when every entry has been replayed.</summary>
        public bool Exhausted => _next == _entries.Count;

        /// <summary>The fixture text: one canonical line per entry.</summary>
        public string ToText()
        {
            var sb = new System.Text.StringBuilder();
            foreach (var e in _entries) sb.Append(e.ToLine()).Append('\n');
            return sb.ToString();
        }

        /// <summary>Runs or replays one value-returning call.</summary>
        public T Do<T>(string call, object?[] args, Func<T> real)
        {
            var argsJson = Norm(JsonSerializer.Serialize(args, Opts));
            if (_recording)
            {
                try
                {
                    var value = real();
                    _entries.Add(new TranscriptEntry(_entries.Count, call, argsJson, Norm(JsonSerializer.Serialize(value, Opts)), null));
                    return value;
                }
                catch (Exception ex)
                {
                    _entries.Add(new TranscriptEntry(_entries.Count, call, argsJson, "null", Norm(ex.Message)));
                    throw;
                }
            }
            var entry = Take(call, argsJson);
            if (entry.Error != null) throw new InvalidOperationException(Denorm(entry.Error));
            return JsonSerializer.Deserialize<T>(Denorm(entry.ResultJson), Opts)!;
        }

        /// <summary>Runs or replays one call that returns nothing.</summary>
        public void Do(string call, object?[] args, Action real) => Do<bool>(call, args, () => { real(); return true; });

        /// <summary>Records the wire exchange (last entry) or checks the replayed response against it byte for byte.</summary>
        public void Exchange(string request, string response)
        {
            var argsJson = JsonSerializer.Serialize(new object[] { Norm(request) }, Opts);
            var resultJson = JsonSerializer.Serialize(Norm(response), Opts);
            if (_recording)
            {
                _entries.Add(new TranscriptEntry(_entries.Count, "rpc", argsJson, resultJson, null));
                return;
            }
            var entry = Take("rpc", argsJson);
            if (entry.ResultJson != resultJson)
            {
                throw new TranscriptMismatchException("seq " + entry.Seq + ": response differs; expected " + entry.ResultJson + ", got " + resultJson);
            }
        }

        private TranscriptEntry Take(string call, string argsJson)
        {
            if (_next >= _entries.Count) throw new TranscriptMismatchException("transcript exhausted; unexpected call " + call + argsJson);
            var entry = _entries[_next];
            if (entry.Call != call || entry.ArgsJson != argsJson)
            {
                throw new TranscriptMismatchException("seq " + entry.Seq + ": expected " + entry.Call + entry.ArgsJson + ", got " + call + argsJson);
            }
            _next++;
            return entry;
        }

        /// <summary>Expands the path token in a scenario request template.</summary>
        public string Expand(string template) => Denorm(template);

        /// <summary>Replaces the per-run temp path in text with the stable {file} token.</summary>
        public string Normalize(string text) => Norm(text);

        private string Norm(string s) => s.Replace(_escapedFile, FileToken);

        private string Denorm(string s) => s.Replace(FileToken, _escapedFile);
    }
}
