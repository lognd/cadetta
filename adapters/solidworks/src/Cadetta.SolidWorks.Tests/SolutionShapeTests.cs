using System;
using System.IO;
using System.Linq;
using Xunit;

namespace Cadetta.SolidWorks.Tests
{
    public class SolutionShapeTests
    {
        private static string SolutionDir()
        {
            var dir = new DirectoryInfo(AppContext.BaseDirectory);
            while (dir != null && !File.Exists(Path.Combine(dir.FullName, "Cadetta.SolidWorks.sln")))
            {
                dir = dir.Parent;
            }
            return dir?.FullName ?? throw new InvalidOperationException("solution not found");
        }

        [Fact]
        public void Solution_has_the_five_projects()
        {
            var sln = File.ReadAllText(Path.Combine(SolutionDir(), "Cadetta.SolidWorks.sln"));
            foreach (var p in new[]
            {
                "Cadetta.SolidWorks.Core", "Cadetta.SolidWorks.Com", "cadetta-solidworks",
                "Cadetta.SolidWorks.AddIn", "Cadetta.SolidWorks.Tests",
            })
            {
                Assert.Contains("\"" + p + "\"", sln);
            }
        }

        [Fact]
        public void Only_Com_and_AddIn_reference_the_interop_assemblies()
        {
            var importing = Directory
                .GetFiles(Path.Combine(SolutionDir(), "src"), "*.csproj", SearchOption.AllDirectories)
                .Where(f => File.ReadAllText(f).Contains("Interop.props") || File.ReadAllText(f).Contains("SolidWorks.Interop"))
                .Select(f => Path.GetFileNameWithoutExtension(f))
                .OrderBy(n => n)
                .ToArray();
            Assert.Equal(new[] { "Cadetta.SolidWorks.AddIn", "Cadetta.SolidWorks.Com" }, importing);
        }

        [Fact]
        public void Interop_assemblies_are_never_committed()
        {
            var dlls = Directory
                .GetFiles(SolutionDir(), "SolidWorks.Interop.*.dll", SearchOption.AllDirectories)
                .Where(f => !f.Contains(Path.DirectorySeparatorChar + "bin" + Path.DirectorySeparatorChar)
                         && !f.Contains(Path.DirectorySeparatorChar + "obj" + Path.DirectorySeparatorChar));
            Assert.Empty(dlls);
        }
    }
}
