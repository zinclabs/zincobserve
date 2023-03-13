<template>
  <div v-if="props.data.type == 'table'" class="q-pa-md">
    <q-table
      class="my-sticky-virtscroll-table"
      virtual-scroll
      v-model:pagination="pagination"
      :rows-per-page-options="[0]"
      :virtual-scroll-sticky-size-start="48"
      dense
      :rows="searchQueryData?.data || []"
      :columns="tableColumn"
      row-key="id"
    >
    </q-table>
  </div>
  <div v-else>
    <div ref="plotRef" :id="chartID" class="plotlycontainer"></div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  onMounted,
  onUpdated,
  ref,
  reactive,
  nextTick,
  watch,
  computed,
  onActivated,
} from "vue";
import { useStore } from "vuex";
import { useQuasar, date } from "quasar";
import queryService from "../../../services/nativequery";
import Plotly from "plotly.js";

export default defineComponent({
  name: "ChartRender",
  props: ["data", "selectedTimeDate"],

  setup(props) {
    const $q = useQuasar();
    const store = useStore();
    const searchQueryData = reactive({
      data: [],
    });

    onUpdated(() => {
      console.log("updated");
      if (props.data.type != "table") {
        renderChart();
      }
    });

    const plotRef: any = ref(null);
    const chartID = ref("chart1");

    let selectedTimeObj = computed(function () {
      return props.selectedTimeDate;
    });

    const tableColumn : any = ref([]);

    // set column value for type chart if the axis value is undefined
    const updateTableColumns = () => {
      const x = props.data?.fields?.x || []
      const y = props.data?.fields?.y || []
      const columnData = [...x, ...y]
      
      const column = columnData.map((it:any)=>{
        let obj : any= {}
        obj["name"] = it.label
        obj["field"] = it.label
        obj["label"] = it.label
        obj["sortable"] = true
        return obj
      })
      tableColumn.value = column
    }

    // If query changes, we need to get the data again and rerender the chart
    watch(
      () => [props.data, props.selectedTimeDate],
      () => {
        if (props.data.query) {
          fetchQueryData();
          updateTableColumns();
        } else {
          Plotly.react(
            plotRef.value,
            [],
            {},
            {
              responsive: true,
              displaylogo: false,
              displayModeBar: false,
            }
          );
        }
      },
      { deep: true }
    );

    // just wait till the component is mounted and then create a plotly instance
    onMounted(async () => {
      await nextTick();
      if (props.data.type != "table") {
        await Plotly.newPlot(
          plotRef.value,
          [{}],
          {},
          {
            responsive: true,
            displaylogo: false,
            displayModeBar: false,
          }
        );
      }

      if (props.data.query) {
        fetchQueryData();
      }
    });

    // wrap the text for long x axis names
    const addBreaksAtLength = 12;
    const textwrapper = function (traces: any) {
      traces = traces.map((text: any) => {
        let rxp = new RegExp(".{1," + addBreaksAtLength + "}", "g");
        if (text) {
          return text?.match(rxp)?.join("<br>");
        } else {
          return " ";
        }
      });
      return traces;
    };

    // Chart Related Functions
    const fetchQueryData = async () => {
      const queryData = props.data.query;
      const chartParams = {
        title: "Found " + "2" + " hits in " + "10" + " ms",
      };

      const sqlQueryModified = queryData;

      // get query object
      // TODO: update queryData to sqlQueryModified
      const timestamps = selectedTimeObj.value;

      let startISOTimestamp: any;
      let endISOTimestamp: any;
      if (
        timestamps.start_time != "Invalid Date" &&
        timestamps.end_time != "Invalid Date"
      ) {
        startISOTimestamp =
          new Date(timestamps.start_time.toISOString()).getTime() * 1000;
        endISOTimestamp =
          new Date(timestamps.end_time.toISOString()).getTime() * 1000;
      }
      const query = {
        query: {
          sql: queryData,
          sql_mode: "full",
          start_time: startISOTimestamp,
          end_time: endISOTimestamp,
        },
      };

      await queryService
        .runquery(query, store.state.selectedOrganization.identifier)
        .then((res) => {
          searchQueryData.data = res.data.hits;

          $q.notify({
            type: "positive",
            message: "Query applied successfully.",
            timeout: 5000,
          });
        })
        .catch((error) => {
          $q.notify({
            type: "negative",
            message: "Something went wrong!",
            timeout: 5000,
          });
        });
    };

    // If data or chart type is updated, rerender the chart
    watch(
      () => [searchQueryData.data, props.data.type],
      () => {
        console.log("Query: new data received");
        if (props.data.type != "table") {
          renderChart();
        }
      },
      { deep: true }
    );

    const renderChart = async () => {
      console.log("Query: rendering chart");
      console.log("Query: chart type", props.data.type);
      // Step 1: Get the Y-Axis Count
      const xAxisKey = getXAxisKey();
      const yAxisKeys = getYAxisKeys();

      console.log("xaxis=", textwrapper(getAxisDataFromKey(xAxisKey)));

      let traces;

      traces = yAxisKeys.map((key: any) => {
        const trace = {
          name: props.data.fields?.y.find((it: any) => it.label == key).column,
          ...getTraceValuesByChartType(xAxisKey, key),
          showlegend: props.data.config?.show_legends,
          marker: {
            color:
              props.data.fields?.y.find((it: any) => it.label == key).color ||
              "#5960b2",
            opacity: 0.8,
          },
        };
        return trace;
      });

      console.log("Query: populating traces: ", traces);

      const layout: any = {
        title: false,
        showlegend: props.data.config?.show_legends,
        font: { size: 12 },
        autosize: true,
        legend: {
          bgcolor: "#f7f7f7",
        },
        xaxis: {
          tickangle: -20,
          automargin: true,
        },
        yaxis: {
          automargin: true,
        },
        margin: {
          l: 32,
          r: 16,
          t: 38,
          b: 32,
        },
        ...getPropsByChartTypeForLayout(),
      };

      Plotly.react(plotRef.value, traces, layout, {
        responsive: true,
        displaylogo: false,
        displayModeBar: false,
      });
    };

    const getTraceValuesByChartType = (xAxisKey: string, yAxisKey: string) => {
      const trace: any = {
        ...getPropsByChartTypeForTraces(),
      };
      if (props.data.type == "pie") {
        trace["labels"] = textwrapper(getAxisDataFromKey(xAxisKey));
        trace["values"] = getAxisDataFromKey(yAxisKey);
      } else if (props.data.type == "h-bar") {
        trace["y"] = textwrapper(getAxisDataFromKey(xAxisKey));
        trace["x"] = getAxisDataFromKey(yAxisKey);
      } else {
        trace["x"] = textwrapper(getAxisDataFromKey(xAxisKey));
        trace["y"] = getAxisDataFromKey(yAxisKey);
      }
      return trace;
    };

    const getXAxisKey = () => {
      return props.data.fields.x.map((it: any) => it.label)[0];
    };

    const getYAxisKeys = () => {
      return props.data.fields.y.map((it: any) => it.label);
    };

    const getAxisDataFromKey = (key: string) => {
      return searchQueryData.data.map((item) => item[key]);
    };

    const getPropsByChartTypeForTraces = () => {
      switch (props.data.type) {
        case "bar":
          return {
            type: "bar",
          };
        case "line":
          return {
            mode: "lines",
          };
        case "scatter":
          return {
            mode: "markers",
          };
        case "pie":
          return {
            type: "pie",
          };
        case "h-bar":
          return {
            type: "bar",
            orientation: "h",
          };
        case "area":
          return {
            fill: "tozeroy", //TODO: hoe to change the color of plot chart
            type: "scatter",
          };
        default:
          return {
            type: "bar",
          };
      }
    };

    const getPropsByChartTypeForLayout = async () => {
      switch (props.data.type) {
        case "bar":
          return {
            barmode: "group",
          };
        case "line":
          return {};
        case "scatter":
          return {
            scattermode: "group",
          };
        case "pie":
          return {};
        case "h-bar":
          return {
            barmode: "group",
          };
        case "area":
          return {};
        default:
          return {};
      }
    };

    return {
      plotRef,
      props,
      searchQueryData,
      pagination: ref({
        rowsPerPage: 0,
      }),
      chartID,
      tableColumn
    };
  },
});
</script>

<style lang="scss" scoped>
.my-sticky-virtscroll-table {
  /* height or max-height is important */
  height: 410px;

  .q-table__top,
  .q-table__bottom,
  thead tr:first-child th {
    /* bg color is important for th; just specify one */
    background-color: #fff;
  }
  thead tr th {
    position: sticky;
    z-index: 1;
  }
  /* this will be the loading indicator */
  thead tr:last-child th {
    /* height of all previous header rows */
    top: 48px;
  }
  thead tr:first-child th {
    top: 0;
  }
}
</style>
