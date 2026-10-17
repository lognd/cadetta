using System.Collections.Generic;
using System.Text;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Save;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Save
{
    public class SaveServiceTests
    {
        private sealed class Doc : ISaveTarget
        {
            public string Path { get; set; } = "a.sldprt";
            public bool Modified { get; set; } = true;
            public bool RebuildFailed { get; set; }
            public string? Fail;
            public int Saves;
            public string? Save() { Saves++; return Fail; }
        }

        private sealed class Reader : IFileReader
        {
            public byte[] ReadAllBytes(string path) => Encoding.ASCII.GetBytes(path);
        }

        private static SaveService Svc() => new SaveService(new Reader(), Log.Null());

        [Fact]
        public void After_sync_saves_modified_files_and_records_digests()
        {
            var doc = new Doc();
            var outcome = Svc().Run(SaveMode.AfterSync, false, new ISaveTarget[] { doc });
            Assert.Equal(1, doc.Saves);
            Assert.Single(outcome.Files);
            Assert.Equal("a.sldprt", outcome.Files[0].Path);
            Assert.Equal(Blake3.Digest(Encoding.ASCII.GetBytes("a.sldprt")), outcome.Files[0].Digest);
            Assert.StartsWith("blake3:", outcome.Files[0].Digest);
        }

        [Fact]
        public void Never_leaves_files_dirty_and_unsaved()
        {
            var doc = new Doc();
            var outcome = Svc().Run(SaveMode.Never, false, new ISaveTarget[] { doc });
            Assert.Equal(0, doc.Saves);
            Assert.True(outcome.Skipped);
            Assert.Empty(outcome.Files);
        }

        [Fact]
        public void Failed_locator_skips_the_save_step()
        {
            var doc = new Doc();
            var outcome = Svc().Run(SaveMode.AfterSync, true, new ISaveTarget[] { doc });
            Assert.Equal(0, doc.Saves);
            Assert.True(outcome.Skipped);
        }

        [Fact]
        public void Only_modified_and_cleanly_rebuilt_files_are_saved()
        {
            var clean = new Doc { Path = "unchanged" , Modified = false };
            var broken = new Doc { Path = "broken", RebuildFailed = true };
            var bad = new Doc { Path = "bad", Fail = "disk full" };
            var outcome = Svc().Run(SaveMode.AfterSync, false, new ISaveTarget[] { clean, broken, bad });
            Assert.Equal(0, clean.Saves + broken.Saves);
            Assert.Empty(outcome.Files);
            Assert.Equal(2, outcome.Diagnostics.Count);
        }

        [Fact]
        public void Save_mode_parses_config_values()
        {
            Assert.Equal(SaveMode.AfterSync, SaveModes.Parse(null));
            Assert.Equal(SaveMode.AfterSync, SaveModes.Parse("after-sync"));
            Assert.Equal(SaveMode.Never, SaveModes.Parse("never"));
            Assert.Null(SaveModes.Parse("sometimes"));
        }

        [Fact]
        public void Blake3_matches_known_vector_for_empty_input()
        {
            Assert.Equal(
                "blake3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
                Blake3.Digest(new byte[0]));
        }
    }
}
