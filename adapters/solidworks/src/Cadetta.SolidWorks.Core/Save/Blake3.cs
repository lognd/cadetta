using System;
using System.Text;

namespace Cadetta.SolidWorks.Save
{
    /// <summary>Unkeyed BLAKE3 with a 32-byte output, enough for file digests.</summary>
    public static class Blake3
    {
        private const int ChunkLen = 1024;
        private const int BlockLen = 64;
        private const uint ChunkStart = 1, ChunkEnd = 2, Parent = 4, Root = 8;

        private static readonly uint[] Iv =
        {
            0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
            0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
        };

        private static readonly int[] Perm = { 2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8 };

        /// <summary>Returns "blake3:" followed by the lowercase hex of the 32-byte hash of <paramref name="data"/>.</summary>
        public static string Digest(byte[] data)
        {
            var words = Hash(data);
            var sb = new StringBuilder("blake3:");
            foreach (var w in words)
            {
                for (var i = 0; i < 4; i++)
                {
                    sb.Append(((w >> (8 * i)) & 0xFF).ToString("x2"));
                }
            }
            return sb.ToString();
        }

        private static uint Rotr(uint x, int n) => (x >> n) | (x << (32 - n));

        private static void G(uint[] s, int a, int b, int c, int d, uint mx, uint my)
        {
            s[a] = s[a] + s[b] + mx;
            s[d] = Rotr(s[d] ^ s[a], 16);
            s[c] = s[c] + s[d];
            s[b] = Rotr(s[b] ^ s[c], 12);
            s[a] = s[a] + s[b] + my;
            s[d] = Rotr(s[d] ^ s[a], 8);
            s[c] = s[c] + s[d];
            s[b] = Rotr(s[b] ^ s[c], 7);
        }

        /// <summary>Compression function; returns the 8-word chaining value (state low half xor high half).</summary>
        private static uint[] Compress(uint[] cv, uint[] block, ulong counter, uint blockLen, uint flags)
        {
            var s = new uint[]
            {
                cv[0], cv[1], cv[2], cv[3], cv[4], cv[5], cv[6], cv[7],
                Iv[0], Iv[1], Iv[2], Iv[3],
                (uint)counter, (uint)(counter >> 32), blockLen, flags,
            };
            var m = (uint[])block.Clone();
            for (var r = 0; r < 7; r++)
            {
                G(s, 0, 4, 8, 12, m[0], m[1]);
                G(s, 1, 5, 9, 13, m[2], m[3]);
                G(s, 2, 6, 10, 14, m[4], m[5]);
                G(s, 3, 7, 11, 15, m[6], m[7]);
                G(s, 0, 5, 10, 15, m[8], m[9]);
                G(s, 1, 6, 11, 12, m[10], m[11]);
                G(s, 2, 7, 8, 13, m[12], m[13]);
                G(s, 3, 4, 9, 14, m[14], m[15]);
                if (r < 6)
                {
                    var p = new uint[16];
                    for (var i = 0; i < 16; i++)
                    {
                        p[i] = m[Perm[i]];
                    }
                    m = p;
                }
            }
            var o = new uint[8];
            for (var i = 0; i < 8; i++)
            {
                o[i] = s[i] ^ s[i + 8];
            }
            return o;
        }

        private static uint[] Hash(byte[] data)
        {
            var chunks = Math.Max(1, (data.Length + ChunkLen - 1) / ChunkLen);
            if (chunks == 1)
            {
                return ChunkCv(data, 0, true);
            }
            var cvs = new uint[chunks][];
            for (var i = 0; i < chunks; i++)
            {
                cvs[i] = ChunkCv(data, i, false);
            }
            return Tree(cvs, 0, chunks, true);
        }

        private static uint[] Tree(uint[][] cvs, int lo, int n, bool root)
        {
            if (n == 1)
            {
                return cvs[lo];
            }
            var left = 1;
            while (left * 2 < n)
            {
                left *= 2;
            }
            var l = Tree(cvs, lo, left, false);
            var r = Tree(cvs, lo + left, n - left, false);
            var block = new uint[16];
            Array.Copy(l, 0, block, 0, 8);
            Array.Copy(r, 0, block, 8, 8);
            return Compress(Iv, block, 0, BlockLen, Parent | (root ? Root : 0));
        }

        private static uint[] ChunkCv(byte[] data, int index, bool root)
        {
            var start = index * ChunkLen;
            var len = Math.Min(ChunkLen, data.Length - start);
            var blocks = Math.Max(1, (len + BlockLen - 1) / BlockLen);
            var cv = (uint[])Iv.Clone();
            for (var b = 0; b < blocks; b++)
            {
                var off = start + b * BlockLen;
                var n = Math.Max(0, Math.Min(BlockLen, start + len - off));
                var words = new uint[16];
                for (var i = 0; i < n; i++)
                {
                    words[i / 4] |= (uint)data[off + i] << (8 * (i % 4));
                }
                var flags = (b == 0 ? ChunkStart : 0u) | (b == blocks - 1 ? ChunkEnd | (root ? Root : 0u) : 0u);
                cv = Compress(cv, words, (ulong)index, (uint)n, flags);
            }
            return cv;
        }
    }
}
