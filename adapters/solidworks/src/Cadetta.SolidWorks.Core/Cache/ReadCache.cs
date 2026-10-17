using System;
using System.Collections.Generic;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Save;

namespace Cadetta.SolidWorks.Cache
{
    /// <summary>
    /// Reuses the observed IR of a file whose on-disk digest is unchanged, so
    /// `read` skips opening it in the tool (adapters.md 2.2).
    /// frob:ticket 01M4SWME4W423G9HT6E2VJJ6M5
    /// </summary>
    public sealed class ReadCache
    {
        private sealed class Entry
        {
            public string Digest = "";
            public Protocol.ReadResult Result = default!;
        }

        private readonly IFileReader _reader;
        private readonly Log _log;
        private readonly Dictionary<string, Entry> _entries = new Dictionary<string, Entry>(StringComparer.Ordinal);

        /// <summary>Creates a cache that digests files through <paramref name="reader"/>.</summary>
        public ReadCache(IFileReader reader, Log log)
        {
            _reader = reader;
            _log = log;
        }

        /// <summary>Number of cached files.</summary>
        public int Count => _entries.Count;

        /// <summary>Records <paramref name="result"/> as the observed IR of <paramref name="path"/> at <paramref name="digest"/> (for example from the lock).</summary>
        public void Record(string path, string digest, Protocol.ReadResult result)
        {
            _entries[path] = new Entry { Digest = digest, Result = result };
            _log.Debug("read cache: recorded " + path + " at " + digest);
        }

        /// <summary>Returns the cached result when the on-disk digest of <paramref name="path"/> matches, else calls <paramref name="open"/> and caches its result.</summary>
        public Protocol.ReadResult GetOrRead(string path, Func<string, Protocol.ReadResult> open)
        {
            var digest = Blake3.Digest(_reader.ReadAllBytes(path));
            if (_entries.TryGetValue(path, out var entry))
            {
                if (entry.Digest == digest)
                {
                    _log.Info("read cache: hit for " + path + ", not opened");
                    return entry.Result;
                }
                _log.Info("read cache: stale for " + path + " (digest changed), reopening");
            }
            else
            {
                _log.Debug("read cache: miss for " + path);
            }
            var result = open(path);
            Record(path, digest, result);
            return result;
        }

        /// <summary>Drops the cached entry for <paramref name="path"/>, if any.</summary>
        public void Invalidate(string path)
        {
            if (_entries.Remove(path))
            {
                _log.Info("read cache: invalidated " + path);
            }
        }
    }
}
