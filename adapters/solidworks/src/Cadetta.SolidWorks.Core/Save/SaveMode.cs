namespace Cadetta.SolidWorks.Save
{
    /// <summary>The `[[adapter]] save` setting: when modified documents are written (adapters.md 2.2).</summary>
    public enum SaveMode
    {
        /// <summary>Save modified documents after a successful regen and read-back (the default).</summary>
        AfterSync,
        /// <summary>Leave documents modified; the sync obligation stays open as "unsaved".</summary>
        Never,
    }

    /// <summary>Parsing for <see cref="SaveMode"/>.</summary>
    public static class SaveModes
    {
        /// <summary>Parses "after-sync" or "never"; null or empty is the default after-sync; anything else is null.</summary>
        public static SaveMode? Parse(string? text)
        {
            if (string.IsNullOrEmpty(text) || text == "after-sync")
            {
                return SaveMode.AfterSync;
            }
            return text == "never" ? SaveMode.Never : (SaveMode?)null;
        }
    }
}
