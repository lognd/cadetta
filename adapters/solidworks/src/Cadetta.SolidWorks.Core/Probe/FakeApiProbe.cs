using System.Collections.Generic;

namespace Cadetta.SolidWorks.Probe
{
    // frob:ticket ~Z9SWC8W
    /// <summary>A probe that returns canned results, for CI and tests.</summary>
    public sealed class FakeApiProbe : ISwApiProbe
    {
        private readonly IReadOnlyList<ApiProbeResult> _results;

        /// <summary>Returns <paramref name="results"/> from every run.</summary>
        public FakeApiProbe(IReadOnlyList<ApiProbeResult> results)
        {
            _results = results;
        }

        /// <summary>The canned results; the scratch directory is ignored.</summary>
        public IReadOnlyList<ApiProbeResult> Run(string scratchDir) => _results;
    }
}
