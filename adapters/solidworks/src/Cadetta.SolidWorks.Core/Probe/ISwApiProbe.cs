using System.Collections.Generic;

namespace Cadetta.SolidWorks.Probe
{
    // frob:ticket 01M4VH6XQF4GRQM20G0Z9SWC8W
    /// <summary>
    /// Exercises each API solidworks.md 4 expects, so a licensed machine can
    /// confirm them. Behind the facade so the COM calls stay in the Com project.
    /// </summary>
    public interface ISwApiProbe
    {
        /// <summary>Runs every probe in a scratch directory and returns one result per probe, in order.</summary>
        IReadOnlyList<ApiProbeResult> Run(string scratchDir);
    }
}
