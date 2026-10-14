using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Session.Recording
{
    /// <summary>
    /// An ISwSession that answers from a recorded transcript, so protocol paths
    /// run in CI without SOLIDWORKS. Calls must arrive in recorded order; any
    /// divergence throws <see cref="TranscriptMismatchException"/>.
    /// </summary>
    public sealed class ReplaySwSession : ISwSession
    {
        private readonly IReadOnlyList<TranscriptEntry> _entries;
        private readonly Log _log;
        private int _next;

        /// <summary>Replays <paramref name="entries"/> in order.</summary>
        public ReplaySwSession(IReadOnlyList<TranscriptEntry> entries, Log log)
        {
            _entries = entries;
            _log = log;
        }

        /// <summary>Loads a transcript (one entry per non-empty line) from <paramref name="reader"/>.</summary>
        public static ReplaySwSession FromReader(TextReader reader, Log log)
        {
            var entries = new List<TranscriptEntry>();
            string? line;
            while ((line = reader.ReadLine()) != null)
            {
                if (line.Length > 0)
                {
                    entries.Add(TranscriptEntry.Parse(line));
                }
            }
            log.Info("loaded transcript with " + entries.Count + " calls");
            return new ReplaySwSession(entries, log);
        }

        /// <summary>True when every recorded call has been replayed.</summary>
        public bool Exhausted => _next == _entries.Count;

        /// <summary>Replays the next recorded call, which must be ProductVersion.</summary>
        public string ProductVersion()
        {
            var entry = Take("ProductVersion", "[]");
            if (entry.Error != null)
            {
                throw new InvalidOperationException(entry.Error);
            }
            return JsonSerializer.Deserialize<string>(entry.ResultJson)
                ?? throw new TranscriptMismatchException("null result for ProductVersion at seq " + entry.Seq);
        }

        private TranscriptEntry Take(string call, string argsJson)
        {
            if (_next >= _entries.Count)
            {
                _log.Error("transcript exhausted at call " + call);
                throw new TranscriptMismatchException("transcript exhausted; unexpected call " + call);
            }
            var entry = _entries[_next];
            if (entry.Call != call || entry.ArgsJson != argsJson)
            {
                _log.Error("transcript mismatch at seq " + entry.Seq);
                throw new TranscriptMismatchException(
                    "seq " + entry.Seq + ": expected " + entry.Call + entry.ArgsJson + ", got " + call + argsJson);
            }
            _next++;
            _log.Write(LogLevel.Trace, "replaying call " + entry.Seq + " " + call);
            return entry;
        }
    }

    /// <summary>The session made a call the transcript did not record at that point.</summary>
    public sealed class TranscriptMismatchException : Exception
    {
        /// <summary>Creates the exception with a description of the divergence.</summary>
        public TranscriptMismatchException(string message) : base(message) { }
    }
}
