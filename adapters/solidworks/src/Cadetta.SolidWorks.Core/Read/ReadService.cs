using System;
using Cadetta.SolidWorks.Cache;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Read
{
    /// <summary>
    /// Implements the `read` method: opens each file through the reader (or the
    /// read cache) and merges their observed IR (adapters.md 5).
    /// frob:ticket 01M4SV2TKZD26CK9FFA1EJXSV4
    /// </summary>
    public sealed class ReadService
    {
        private readonly INativeDocumentReader _reader;
        private readonly ReadCache? _cache;
        private readonly Log _log;

        /// <summary>Creates a service over <paramref name="reader"/>, optionally reusing <paramref name="cache"/>.</summary>
        public ReadService(INativeDocumentReader reader, ReadCache? cache, Log log)
        {
            _reader = reader;
            _cache = cache;
            _log = log;
        }

        /// <summary>Reads one SOLIDWORKS file into its observed IR, through the cache when present.</summary>
        public Protocol.ReadResult Read(string file)
        {
            _log.Info("read " + file);
            Func<string, Protocol.ReadResult> open = path =>
            {
                var b = new ObservedIrBuilder(path, _log);
                b.Add(_reader.Read(path));
                return b.Build();
            };
            return _cache == null ? open(file) : _cache.GetOrRead(file, open);
        }
    }
}
