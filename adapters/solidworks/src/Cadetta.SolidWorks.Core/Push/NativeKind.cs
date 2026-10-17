namespace Cadetta.SolidWorks.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>The kind of native item a locator names, as the document resolves it (adapters.md 3).</summary>
    public enum NativeKind
    {
        /// <summary>An equation-manager global variable.</summary>
        Global,

        /// <summary>A dimension such as D1@Boss-Extrude1.</summary>
        Dimension,

        /// <summary>A custom property.</summary>
        Property,

        /// <summary>A coordinate system feature.</summary>
        Frame,
    }
}
