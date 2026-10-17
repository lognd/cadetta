using System.Collections.Generic;
using System.Text;
using Cadetta.SolidWorks.Cache;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Save;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Cache
{
    public class ReadCacheTests
    {
        private sealed class Files : IFileReader
        {
            public Dictionary<string, string> Content = new Dictionary<string, string>();
            public byte[] ReadAllBytes(string path) => Encoding.ASCII.GetBytes(Content[path]);
        }

        private static Protocol.ReadResult Result() => new Protocol.ReadResult();

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Cache/ReadCache.cs::ReadCache.GetOrRead kind=unit
        [Fact]
        public void Unchanged_digest_returns_cached_result_without_opening()
        {
            var files = new Files();
            files.Content["a.sldprt"] = "v1";
            var cache = new ReadCache(files, Log.Null());
            var opens = 0;
            var first = cache.GetOrRead("a.sldprt", p => { opens++; return Result(); });
            var second = cache.GetOrRead("a.sldprt", p => { opens++; return Result(); });
            Assert.Equal(1, opens);
            Assert.Same(first, second);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Cache/ReadCache.cs::ReadCache.GetOrRead kind=unit
        [Fact]
        public void Changed_digest_reopens_and_replaces_entry()
        {
            var files = new Files();
            files.Content["a.sldprt"] = "v1";
            var cache = new ReadCache(files, Log.Null());
            var opens = 0;
            var first = cache.GetOrRead("a.sldprt", p => { opens++; return Result(); });
            files.Content["a.sldprt"] = "v2";
            var second = cache.GetOrRead("a.sldprt", p => { opens++; return Result(); });
            Assert.Equal(2, opens);
            Assert.NotSame(first, second);
            Assert.Equal(1, cache.Count);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Cache/ReadCache.cs::ReadCache.Record kind=unit
        [Fact]
        public void Recorded_lock_digest_serves_first_read()
        {
            var files = new Files();
            files.Content["a.sldprt"] = "v1";
            var cache = new ReadCache(files, Log.Null());
            var recorded = Result();
            cache.Record("a.sldprt", Blake3.Digest(Encoding.ASCII.GetBytes("v1")), recorded);
            var got = cache.GetOrRead("a.sldprt", p => throw new System.InvalidOperationException("opened"));
            Assert.Same(recorded, got);
        }

        // frob:tests adapters/solidworks/src/Cadetta.SolidWorks.Core/Cache/ReadCache.cs::ReadCache.Invalidate kind=unit
        [Fact]
        public void Invalidate_forces_reopen()
        {
            var files = new Files();
            files.Content["a.sldprt"] = "v1";
            var cache = new ReadCache(files, Log.Null());
            var opens = 0;
            cache.GetOrRead("a.sldprt", p => { opens++; return Result(); });
            cache.Invalidate("a.sldprt");
            cache.GetOrRead("a.sldprt", p => { opens++; return Result(); });
            Assert.Equal(2, opens);
        }
    }
}
