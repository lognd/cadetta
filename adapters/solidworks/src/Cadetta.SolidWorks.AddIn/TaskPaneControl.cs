// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
// frob:ticket 01M54BDDBT01RBBDS3AQ8ZNA4K
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.AddIn.Pane;

namespace Cadetta.SolidWorks.AddIn
{
    /// <summary>The Cadetta task pane: a status line, a button bar for the link, sync and why commands, and a list of the document component's paths (solidworks.md 2.1, 3).</summary>
    [ComVisible(true)]
    [Guid("6f1d3a52-9b0c-4c5e-8a47-5d2e0c7b1a93")]
    [ProgId(ProgIdText)]
    [ClassInterface(ClassInterfaceType.None)]
    public sealed class TaskPaneControl : UserControl
    {
        /// <summary>COM ProgId used by CreateTaskpaneView2.AddControl.</summary>
        public const string ProgIdText = "Cadetta.SolidWorks.TaskPane";

        private readonly Label _status = new Label { Dock = DockStyle.Top, Height = 36, AutoEllipsis = true };
        /// <summary>Command names raised by <see cref="CommandRequested"/>.</summary>
        public static readonly string[] Verbs = { "Link", "Unlink", "Preview", "Sync", "Why", "AssignArray" };

        private readonly FlowLayoutPanel _bar = new FlowLayoutPanel { Dock = DockStyle.Top, AutoSize = true, WrapContents = true };
        private readonly ListView _list = new ListView { Dock = DockStyle.Fill, View = View.Details, FullRowSelect = true, HideSelection = false };

        /// <summary>Creates the empty pane; the add-in attaches a model after SOLIDWORKS builds the control.</summary>
        public TaskPaneControl()
        {
            foreach (var h in new[] { "Path", "Value", "Tolerance", "Citation", "Doc" })
            {
                _list.Columns.Add(h, h == "Path" ? 160 : 80);
            }
            foreach (var verb in Verbs)
            {
                var name = verb;
                var button = new Button { Text = verb == "Preview" ? "Sync preview" : verb == "AssignArray" ? "Assign array" : verb, AutoSize = true };
                button.Click += (s, e) => CommandRequested?.Invoke(name);
                _bar.Controls.Add(button);
            }
            Controls.Add(_list);
            Controls.Add(_bar);
            Controls.Add(_status);
        }

        /// <summary>Raised with one of <see cref="Verbs"/> when the engineer presses a command button.</summary>
        public event Action<string>? CommandRequested;

        /// <summary>The path of the selected row, or null when no row is selected.</summary>
        public string? SelectedPath => _list.SelectedItems.Count == 0 ? null : (_list.SelectedItems[0].Tag as NameRow)?.Path;

        /// <summary>Shows a command's result text in the status line.</summary>
        public void ShowResult(string text)
        {
            _status.Text = text;
        }

        /// <summary>Shows the model's status and the rows of the current selection view.</summary>
        public void Render(string status, IReadOnlyList<NameRow> rows)
        {
            _status.Text = status;
            _list.BeginUpdate();
            _list.Items.Clear();
            foreach (var r in rows)
            {
                var item = new ListViewItem(r.Path);
                item.SubItems.Add(r.Emit ?? "");
                item.SubItems.Add(r.Tolerance ?? "");
                item.SubItems.Add(r.Citation ?? "");
                item.SubItems.Add(r.Doc ?? "");
                item.Tag = r;
                _list.Items.Add(item);
            }
            _list.EndUpdate();
        }
    }
}
