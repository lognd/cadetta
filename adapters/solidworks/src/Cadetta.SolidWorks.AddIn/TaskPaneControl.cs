// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.AddIn.Pane;

namespace Cadetta.SolidWorks.AddIn
{
    /// <summary>The Cadetta task pane: a status line and a list of the document component's paths (solidworks.md 2.1, 3).</summary>
    [ComVisible(true)]
    [Guid("6f1d3a52-9b0c-4c5e-8a47-5d2e0c7b1a93")]
    [ProgId(ProgIdText)]
    [ClassInterface(ClassInterfaceType.None)]
    public sealed class TaskPaneControl : UserControl
    {
        /// <summary>COM ProgId used by CreateTaskpaneView2.AddControl.</summary>
        public const string ProgIdText = "Cadetta.SolidWorks.TaskPane";

        private readonly Label _status = new Label { Dock = DockStyle.Top, Height = 36, AutoEllipsis = true };
        private readonly ListView _list = new ListView { Dock = DockStyle.Fill, View = View.Details, FullRowSelect = true, HideSelection = false };

        /// <summary>Creates the empty pane; the add-in attaches a model after SOLIDWORKS builds the control.</summary>
        public TaskPaneControl()
        {
            foreach (var h in new[] { "Path", "Value", "Tolerance", "Citation", "Doc" })
            {
                _list.Columns.Add(h, h == "Path" ? 160 : 80);
            }
            Controls.Add(_list);
            Controls.Add(_status);
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
