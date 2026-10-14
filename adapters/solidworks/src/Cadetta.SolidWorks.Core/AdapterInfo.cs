namespace Cadetta.SolidWorks
{
    /// <summary>Identity the adapter reports at initialize (adapters.md 5).</summary>
    public static class AdapterInfo
    {
        /// <summary>Adapter name.</summary>
        public const string Name = "solidworks";

        /// <summary>Adapter version; must equal the host's [[adapter]] version (R008).</summary>
        public const string Version = "0.0.0";

        /// <summary>The tool's native length unit.</summary>
        public const string NativeUnits = "mm";

        /// <summary>Fractional decimal digits of the native number format (adapters.md 5 default).</summary>
        public const uint Precision = 6;
    }
}
