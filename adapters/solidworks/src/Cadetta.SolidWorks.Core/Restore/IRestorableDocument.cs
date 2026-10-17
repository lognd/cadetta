namespace Cadetta.SolidWorks.Restore
{
    /// <summary>One open document seen as named numeric locators plus a rebuild, the surface snapshot and restore needs (adapters.md 2.3).</summary>
    public interface IRestorableDocument
    {
        /// <summary>Reads the current value of <paramref name="locator"/>; null when the locator does not resolve.</summary>
        double? GetValue(string locator);

        /// <summary>Writes <paramref name="value"/> to <paramref name="locator"/>; false when the write is refused.</summary>
        bool SetValue(string locator, double value);

        /// <summary>Rebuilds the document; false when the rebuild reports errors.</summary>
        bool Rebuild();
    }
}
