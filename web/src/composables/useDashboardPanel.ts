import { reactive } from "vue";
const getDefaultDashboardPanelData = () => (
  {
    data: {
      id: "",
      type: "bar",
      fields: {
        stream: '',
        x: <any>[],
        y: <any>[],
        filter: <any>[]
      },
      config: {
        title: "",
        description: "",
        show_legends: false,
      },
      query: "",
    },
    layout: {
      showCustomQuery: false,
      loading:false,
    },
    meta: {
      parsedQuery: "",
      editorValue: "",
      dateTime: {},
      filterValue: <any>[],
      stream: {
        selectedStreamFields: [],
        customQueryFields: [],
        functions: [],
        streamResults: <any>[],
        filterField: "",
      },
    }
  }
)

let dashboardPanelData = reactive({ ...getDefaultDashboardPanelData()});

const useDashboardPanelData = () => {
  const resetDashboardPanelData = () => {
    Object.assign(dashboardPanelData, getDefaultDashboardPanelData());
    console.log("updated...",dashboardPanelData);
  };

  const addXAxisItem = (name: string) => {
    if(!dashboardPanelData.data.fields.x) {
      dashboardPanelData.data.fields.x = []
    }

    // TODO: condition for all chart type
    if(dashboardPanelData.data.fields.x.length >= 1){
      return;
    } 

    // check for existing field
    if(!dashboardPanelData.data.fields.x.find((it:any) => it.column == name)) {
      dashboardPanelData.data.fields.x.push({
        label: 'x_axis_' + dashboardPanelData.data.fields.x.length,
        column: name,
        color: null,
        aggregationFunction: null
      })
    }
  }

  const addYAxisItem = (name: string) => {
    if(!dashboardPanelData.data.fields.y) {
      dashboardPanelData.data.fields.y = []
    }

    if(!dashboardPanelData.data.fields.y.find((it:any) => it.column == name)) {
      dashboardPanelData.data.fields.y.push({
        label: 'y_axis_' + dashboardPanelData.data.fields.y.length,
        column: name,
        color: '#5960b2',
        aggregationFunction: 'count'
      })
    }
  }


  const removeXAxisItem = (name: string) => {
    const index = dashboardPanelData.data.fields.x.findIndex((it:any) => it.column == name)
    if(index >= 0) {
      dashboardPanelData.data.fields.x.splice(index, 1)
    }
  }

  const removeYAxisItem = (name: string) => {
    const index = dashboardPanelData.data.fields.y.findIndex((it:any) => it.column == name)
    if(index >= 0) {
      dashboardPanelData.data.fields.y.splice(index, 1)
    }
  }

  const removeFilterItem = (name: string) => {
    const index = dashboardPanelData.data.fields.filter.findIndex((it:any) => it.column == name)
    if(index >= 0) {
      dashboardPanelData.data.fields.filter.splice(index, 1)
    }
  }
  return { 
    dashboardPanelData, 
    resetDashboardPanelData, 
    addXAxisItem, 
    addYAxisItem,
    removeXAxisItem,
    removeYAxisItem,
    removeFilterItem
  };
};

export default useDashboardPanelData;
