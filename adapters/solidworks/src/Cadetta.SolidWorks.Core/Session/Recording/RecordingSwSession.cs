using System;
using System.IO;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Session.Recording
{
    /// <summary>
    /// Wraps a real ISwSession and writes every call and result to a transcript
    /// as JSON lines, for later replay by <see cref="ReplaySwSession"/> (solidworks.md 6).
    /// </summary>
    public sealed class RecordingSwSession : ISwSession
    {
        private readonly ISwSession _inner;
        private readonly TextWriter _sink;
        private readonly Log _log;
        private int _seq;

        /// <summary>Records calls on <paramref name="inner"/> to <paramref name="sink"/> (newline-terminated lines).</summary>
        public RecordingSwSession(ISwSession inner, TextWriter sink, Log log)
        {
            _inner = inner;
            _sink = sink;
            _log = log;
        }

        /// <summary>Forwards to the wrapped session and records the call.</summary>
        public string ProductVersion()
        {
            return Record("ProductVersion", "[]", () => _inner.ProductVersion());
        }

        private string Record(string call, string argsJson, Func<string> invoke)
        {
            var seq = _seq++;
            try
            {
                var result = invoke();
                Write(new TranscriptEntry(seq, call, argsJson, System.Text.Json.JsonSerializer.Serialize(result), null));
                return result;
            }
            catch (Exception ex)
            {
                Write(new TranscriptEntry(seq, call, argsJson, "null", ex.Message));
                throw;
            }
        }

        private void Write(TranscriptEntry entry)
        {
            _log.Write(LogLevel.Trace, "recording call " + entry.Seq + " " + entry.Call);
            _sink.Write(entry.ToLine());
            _sink.Write('\n');
            _sink.Flush();
        }
    }
}
