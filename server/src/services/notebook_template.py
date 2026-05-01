import marimo

__generated_by__ = "tilt-app"

app = marimo.App(width="full", app_title="__BREW_NAME__")


@app.cell
def _():
    import marimo as mo
    import pandas as pd
    import plotly.graph_objects as go
    from datetime import datetime
    return mo, pd, go, datetime


@app.cell
def _(mo):
    mo.md(
        r"""
        # __BREW_NAME__

        **Status:** __BREW_STATUS__
        **Style:** __BREW_STYLE__
        """
    )
    return


@app.cell
def _():
    brew = {
        "id": "__BREW_ID__",
        "name": "__BREW_NAME__",
        "status": "__BREW_STATUS__",
        "og": __BREW_OG__,
        "fg": __BREW_FG__,
        "target_fg": __BREW_TARGET_FG__,
        "live_abv": __BREW_LIVE_ABV__,
        "final_abv": __BREW_FINAL_ABV__,
        "start_date": __BREW_START_DATE__,
        "end_date": __BREW_END_DATE__,
        "notes": __BREW_NOTES__,
    }
    return (brew,)


@app.cell
def _():
    import pandas as pd
    _readings_raw = [
__READINGS_DATA__
    ]
    readings = pd.DataFrame(_readings_raw)
    if not readings.empty:
        readings["recorded_at"] = pd.to_datetime(readings["recorded_at"], utc=True)
        readings = readings.sort_values("recorded_at").reset_index(drop=True)
    readings
    return (readings,)


@app.cell
def _(mo, readings, brew, go):
    if readings.empty:
        mo.md("_No readings recorded yet._")
    else:
        fig = go.Figure()

        fig.add_trace(go.Scatter(
            x=readings["recorded_at"],
            y=readings["gravity"],
            mode="lines",
            name="Gravity (SG)",
            line={"color": "#3b82f6"},
            yaxis="y1",
        ))

        fig.add_trace(go.Scatter(
            x=readings["recorded_at"],
            y=readings["temperature_f"],
            mode="lines",
            name="Temperature (F)",
            line={"color": "#f97316", "dash": "dot"},
            yaxis="y2",
        ))

        if brew.get("target_fg") is not None:
            fig.add_hline(
                y=brew["target_fg"],
                line_dash="dash",
                line_color="green",
                annotation_text="Target FG {:.3f}".format(brew["target_fg"]),
                yref="y1",
            )

        if brew.get("og") is not None:
            fig.add_hline(
                y=brew["og"],
                line_dash="dash",
                line_color="gray",
                annotation_text="OG {:.3f}".format(brew["og"]),
                yref="y1",
            )

        fig.update_layout(
            title="__BREW_NAME__ — Fermentation",
            xaxis_title="Time",
            yaxis={"title": "Gravity (SG)", "side": "left"},
            yaxis2={"title": "Temperature (F)", "side": "right", "overlaying": "y"},
            legend={"orientation": "h"},
            height=500,
        )

        mo.ui.plotly(fig)
    return


@app.cell
def _(mo, readings, brew):
    if not readings.empty:
        current_g = float(readings["gravity"].iloc[-1])
        n = len(readings)
        items = [
            mo.stat(label="Readings", value=str(n)),
            mo.stat(label="Current Gravity", value="{:.3f}".format(current_g)),
            mo.stat(label="Min Gravity", value="{:.3f}".format(float(readings["gravity"].min()))),
            mo.stat(label="Max Temp (F)", value="{:.1f}".format(float(readings["temperature_f"].max()))),
        ]
        if brew.get("og") and brew.get("target_fg"):
            og = brew["og"]
            apparent_atten = (og - current_g) / (og - 1.0) * 100
            items.append(mo.stat(label="Apparent Attenuation", value="{:.1f}%".format(apparent_atten)))
        mo.hstack(items)
    return


if __name__ == "__main__":
    app.run()
