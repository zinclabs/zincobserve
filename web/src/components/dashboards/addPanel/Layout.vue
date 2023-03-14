<template>
  <div>
    <div class="q-pa-sm">
      <span class="text-weight-bold">Layout</span>
    </div>
    <q-separator />
    <div>
      <q-expansion-item
        default-opened
        dense
        :label="
          dashboardPanelData.data.type == 'table'
            ? t(`First Column`)
            : dashboardPanelData.data.type == 'h-bar'
            ? t(`Y-Axis (Columns)`)
            : t(`X-Axis (Row)`)
        "
      >
        <div class="column index-menu q-mb-md">
          <div class="index-table q-mt-xs">
            <q-table
              v-model:selected="dashboardPanelData.data.fields.x"
              :columns="[
                {
                  name: 'column',
                  field: 'column',
                  align: 'left',
                  label: 'Field',
                },
              ]"
              :rows="dashboardPanelData.data.fields.x"
              row-key="name"
              class="field-table"
              id="fieldList"
              :hide-pagination="true"
              hide-header
              hide-bottom
            >
              <template #body-cell-column="props">
                <q-tr :props="props">
                  <q-td :props="props" class="field_list">
                    <div class="field_overlay">
                      <div class="field_label">
                        {{ props.row.column }}
                      </div>
                      <div class="field_icons">
                        <q-icon
                          :name="
                            'img:' +
                            getImageURL('images/layout/remove_icon.svg')
                          "
                          size="1rem"
                          @click="removeXAxisItem(props.row.column)"
                        />
                      </div>
                    </div>
                  </q-td>
                </q-tr>
              </template>
            </q-table>
          </div>
          <div
            class="text-caption text-weight-bold text-center q-ma-sm"
            v-if="dashboardPanelData.data.fields.x.length < 1"
          >
            Please add a field from the list
          </div>
        </div>
      </q-expansion-item>
      <q-separator />
      <q-expansion-item
        dense
        default-opened
        :label="
          dashboardPanelData.data.type == 'table'
            ? t(`Other Columns`)
            : dashboardPanelData.data.type == 'h-bar'
            ? t(`X-Axis (Row)`)
            : t(`Y-Axis (Columns)`)
        "
      >
        <div class="column index-menu q-mb-lg">
          <div class="index-table q-mt-xs">
            <q-table
              v-model:selected="dashboardPanelData.data.fields.y"
              :columns="[
                {
                  name: 'column',
                  field: 'column',
                  align: 'left',
                  label: 'Field',
                },
              ]"
              :rows="dashboardPanelData.data.fields.y"
              row-key="column"
              class="field-table"
              id="fieldList"
              virtual-scroll
              v-model:pagination="pagination"
              :rows-per-page-options="[0]"
              :virtual-scroll-sticky-size-start="48"
              :hide-pagination="true"
              hide-header
              hide-bottom
            >
              <template #body-cell-column="props">
                <q-tr :props="props">
                  <q-td :props="props" class="field_list">
                    <div class="field_overlay" :title="props.row.column">
                      <div class="field_label">
                        {{ props.row.column }}
                        <!-- <div>
                        <q-expansion-item
                          expand-separator
                          icon="perm_identity"
                          label=""
                          caption=""
                        >
                          <div class="color-input-wrapper">
                            <input type="color" v-model="dashboardPanelData.data.fields.y.color">
                          </div>
                          <div>
                            <q-select
                              v-model="dashboardPanelData.data.fields.y.aggregationFunction"
                              :options="triggerOperators"
                              dense
                              filled
                              :label="t('Aggregation')"
                            ></q-select>
                          </div>
                        </q-expansion-item>
                      </div> -->
                      </div>

                      <div class="field_icons">
                        <!-- TODO: aggregation and color picker and label fields -->

                        <div>
                          <q-btn
                            size="xs"
                            color="red"
                            round
                            dense
                            @click="props.expand = !props.expand"
                            :icon="props.expand ? 'remove' : 'add'"
                          />
                          <q-icon
                            :name="
                              'img:' +
                              getImageURL('images/layout/remove_icon.svg')
                            "
                            size="1rem"
                            @click="removeYAxisItem(props.row.column)"
                          />
                        </div>
                      </div>
                    </div>
                  </q-td>
                </q-tr>
                <q-tr
                  v-show="props.expand"
                  :props="props"
                  style="height: min-content"
                >
                  <q-td colspan="100%">
                    <div>
                      <div class="flex items-center">
                        <div class="q-pa-xs" style="width: 180px">
                          <q-select
                            v-model="
                              dashboardPanelData.data.fields.y[props.pageIndex]
                                .aggregationFunction
                            "
                            :options="triggerOperators"
                            dense
                            filled
                            label="Aggregation"
                          ></q-select>
                        </div>
                        <div class="color-input-wrapper" v-if="!['table', 'pie'].includes(dashboardPanelData.data.type)">
                          <input
                            type="color"
                            v-model="
                              dashboardPanelData.data.fields.y[props.pageIndex]
                                .color
                            "
                          />
                        </div>
                      </div>
                      <q-input
                        dense
                        filled
                        label="Label"
                        v-model="
                          dashboardPanelData.data.fields.y[props.pageIndex]
                            .label
                        "
                      />
                    </div>
                  </q-td>
                </q-tr>
              </template>
            </q-table>
          </div>
          <div
            class="text-caption text-weight-bold text-center q-ma-sm"
            v-if="dashboardPanelData.data.fields.y.length < 1"
          >
            Please add a field from the list
          </div>
        </div>
      </q-expansion-item>
      <q-separator />
      <q-expansion-item default-opened dense :label="t(`Config`)">
        <div>
          <q-toggle
            v-if="dashboardPanelData.data.type != 'table'"
            v-model="dashboardPanelData.data.config.show_legends"
            label="Show Legends"
          />
          <q-form ref="" class="q-pa-sm">
            <q-input
              v-model="dashboardPanelData.data.config.title"
              :label="t('panel.name') + '*'"
              class="q-py-md showLabelOnTop"
              stack-label
              filled
              dense
            />
            <q-input
              v-model="dashboardPanelData.data.config.description"
              type="textarea"
              :label="t('panel.typeDesc')"
              class="q-py-md showLabelOnTop"
              stack-label
              filled
              dense
            />
          </q-form>
        </div>
      </q-expansion-item>
    </div>
    <q-separator />
    <q-expansion-item default-opened dense :label="t(`Filters`)">
      <div class="column index-menu q-mb-lg">
        <div class="index-table q-mt-xs">
          <q-table
            v-model:selected="dashboardPanelData.data.fields.filter"
            :columns="[
              {
                name: 'column',
                field: 'column',
                align: 'left',
                label: 'Filter',
              },
            ]"
            :rows="dashboardPanelData.data.fields.filter"
            row-key="column"
            class="field-table"
            id="fieldList"
            virtual-scroll
            v-model:pagination="pagination"
            :rows-per-page-options="[0]"
            :virtual-scroll-sticky-size-start="48"
            :hide-pagination="true"
            hide-header
            hide-bottom
          >
            <template #body-cell-column="props">
              <q-tr :props="props">
                <q-td :props="props" class="field_list">
                  <div class="field_overlay" :title="props.row.column">
                    <div class="field_label">
                      {{ props.row.column }}
                    </div>
                    <div class="field_icons">
                      <div>
                        <q-btn
                          size="xs"
                          color="red"
                          round
                          dense
                          @click="props.expand = !props.expand"
                          :icon="props.expand ? 'remove' : 'add'"
                        />
                        <q-icon
                          :name="
                            'img:' +
                            getImageURL('images/layout/remove_icon.svg')
                          "
                          size="1rem"
                          @click="removeFilterItem(props.row.column)"
                        />
                      </div>
                    </div>
                  </div>
                </q-td>
              </q-tr>
              <q-tr
                v-show="props.expand"
                :props="props"
                style="height: min-content"
              >
                <q-td colspan="100%">
                  <div class="q-pa-xs">
                    <div class="q-gutter-xs">
                      <q-tabs
                        v-model="
                          dashboardPanelData.data.fields.filter[props.pageIndex]
                            .type
                        "
                      >
                        <q-tab
                          name="list"
                          label="List"
                          style="width: auto"
                        ></q-tab>
                        <q-tab
                          name="condition"
                          label="Condition"
                          style="width: auto"
                        ></q-tab>
                      </q-tabs>
                      <q-separator></q-separator>
                      <q-tab-panels
                        dense
                        v-model="
                          dashboardPanelData.data.fields.filter[props.pageIndex]
                            .type
                        "
                        animated
                        style="background-color: #f5f5f5"
                      >
                        <q-tab-panel dense name="condition">
                          <div class="flex justify-between">
                            <q-select
                              dense
                              filled
                              v-model="
                                dashboardPanelData.data.fields.filter[
                                  props.pageIndex
                                ].operator
                              "
                              :options="options"
                              label="Operator"
                              style="width: 100%"
                            />
                            <q-input
                              dense
                              filled
                              v-model="
                                dashboardPanelData.data.fields.filter[
                                  props.pageIndex
                                ].value
                              "
                              label="Value"
                              style="width: 100%; margin-top: 5px"
                            />
                          </div>
                        </q-tab-panel>
                        <q-tab-panel dense name="list">
                          <q-select
                            dense
                            filled
                            v-model="
                              dashboardPanelData.data.fields.filter[
                                props.pageIndex
                              ].values
                            "
                            :options="dashboardPanelData.meta.filterValue.find((it: any)=>it.column == props.row.column)?.value"
                            label="Select Filter"
                            multiple
                            emit-value
                            map-options
                          >
                            <template v-slot:selected>
                              {{
                                dashboardPanelData.data.fields.filter[
                                  props.pageIndex
                                ].values[0]?.length > 15
                                  ? dashboardPanelData.data.fields.filter[
                                      props.pageIndex
                                    ].values[0]?.substring(0, 15) + "..."
                                  : dashboardPanelData.data.fields.filter[
                                      props.pageIndex
                                    ].values[0]
                              }}

                              {{
                                dashboardPanelData.data.fields.filter[
                                  props.pageIndex
                                ].values?.length > 1
                                  ? " +" +
                                    (dashboardPanelData.data.fields.filter[
                                      props.pageIndex
                                    ].values?.length -
                                      1)
                                  : ""
                              }}
                            </template>
                            <template
                              v-slot:option="{
                                itemProps,
                                opt,
                                selected,
                                toggleOption,
                              }"
                            >
                              <q-item v-bind="itemProps">
                                <q-item-section side>
                                  <q-checkbox
                                    dense
                                    :model-value="selected"
                                    @update:model-value="toggleOption(opt)"
                                  ></q-checkbox>
                                </q-item-section>
                                <q-item-section>
                                  <div v-html="opt"></div>
                                </q-item-section>
                              </q-item>
                            </template>
                          </q-select>
                        </q-tab-panel>
                      </q-tab-panels>
                    </div>
                  </div>
                </q-td>
              </q-tr>
            </template>
          </q-table>
        </div>
        <div
          class="text-caption text-weight-bold text-center q-ma-sm"
          v-if="dashboardPanelData.meta.filterValue.length < 1"
        >
          Please add a field from the list
        </div>
      </div>
      <div></div>
    </q-expansion-item>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, reactive } from "vue";
import { useI18n } from "vue-i18n";
import { useQuasar } from "quasar";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import { getImageURL } from "../../../utils/zincutils";

export default defineComponent({
  name: "dashboard-layout",
  components: {},
  setup() {
    const showXAxis = ref(true);
    const panelName = ref("");
    const panelDesc = ref("");
    const { t } = useI18n();
    const $q = useQuasar();
    const {
      dashboardPanelData,
      removeXAxisItem,
      removeYAxisItem,
      removeFilterItem,
    } = useDashboardPanelData();
    const triggerOperators: any = ref(["count", "sum", "avg", "min", "max"]);

    return {
      showXAxis,
      t,
      panelName,
      panelDesc,
      dashboardPanelData,
      removeXAxisItem,
      removeYAxisItem,
      triggerOperators,
      removeFilterItem,
      pagination: ref({
        rowsPerPage: 0,
      }),
      model: ref([]),
      tab: ref("General"),
      options: ["=", "<>", ">=", "<=", ">", "<", "Contains", "Not Contains"],
      getImageURL,
    };
  },
});
</script>

<style lang="scss" scoped>
.color-input-wrapper {
  height: 1.5em;
  width: 1.5em;
  overflow: hidden;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  position: relative;
}
.color-input-wrapper input[type="color"] {
  position: absolute;
  height: 4em;
  width: 4em;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  overflow: hidden;
  border: none;
  margin: 0;
  padding: 0;
}
.q-menu {
  box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
  transform: translateY(0.5rem);
  border-radius: 0px;

  .q-virtual-scroll__content {
    padding: 0.5rem;
  }
}
.index-menu {
  width: 100%;

  .q-field {
    &__control {
      height: 35px;
      padding: 0px 5px;
      min-height: auto !important;

      &-container {
        padding-top: 0px !important;
      }
    }
    &__native :first-of-type {
      padding-top: 0.25rem;
    }
  }

  .q-select {
    text-transform: capitalize;
  }

  .index-table {
    width: 100%;
    // border: 1px solid rgba(0, 0, 0, 0.02);
    .q-table {
      display: block;
    }
    tr {
      margin-bottom: 1px;
    }
    tbody,
    tr,
    td {
      width: 100%;
      display: block;
      height: 25px;
    }

    .q-table__top {
      padding: 0px;
    }
    .q-table__control,
    label.q-field {
      width: 100%;
    }
    .q-table thead tr,
    .q-table tbody td {
      height: auto;
    }

    .q-table__top {
      border-bottom: unset;
    }
  }
  .field-table {
    width: 100%;
  }

  .field_list {
    padding: 0px;
    margin-bottom: 0.125rem;
    position: relative;
    overflow: visible;
    cursor: default;

    .field_overlay {
      justify-content: space-between;
      background-color: transparent;
      transition: all 0.3s ease;
      padding: 0px 10px;
      align-items: center;
      position: absolute;
      line-height: 2rem;
      overflow: hidden;
      inset: 0;
      display: flex;
      z-index: 1;
      width: 100%;
      border-radius: 0px;
      height: 25px;

      .field_icons {
        padding: 0 0.625rem 0 0.25rem;
        transition: all 0.3s ease;
        background-color: white;
        position: absolute;
        z-index: 3;
        opacity: 0;
        right: 0;

        .q-icon {
          cursor: pointer;
        }
      }

      .field_label {
        pointer-events: none;
        font-size: 0.825rem;
        position: relative;
        display: inline;
        z-index: 2;
        left: 0;
        // text-transform: capitalize;
      }
    }

    &.selected {
      .field_overlay {
        background-color: rgba(89, 96, 178, 0.3);

        .field_icons {
          opacity: 0;
        }
      }
      &:hover {
        .field_overlay {
          box-shadow: 0px 4px 15px rgba(0, 0, 0, 0.17);
          background-color: white;

          .field_icons {
            background-color: white;
          }
        }
      }
    }
    &:hover {
      .field_overlay {
        box-shadow: 0px 4px 15px rgba(0, 0, 0, 0.17);

        .field_icons {
          background-color: white;
          opacity: 1;
        }
      }
    }
  }
}
.q-item {
  color: $dark-page;
  min-height: 1.3rem;
  padding: 5px 10px;

  &__label {
    font-size: 0.75rem;
  }

  &.q-manual-focusable--focused > .q-focus-helper {
    background: none !important;
    opacity: 0.3 !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &--active {
    background-color: $selected-list-bg !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &:hover,
  &--active {
    color: $primary;
  }
}
.q-field--dense .q-field__before,
.q-field--dense .q-field__prepend {
  padding: 0px 0px 0px 0px;
  height: auto;
  line-height: auto;
}
.q-field__native,
.q-field__input {
  padding: 0px 0px 0px 0px;
}

.q-field--dense .q-field__label {
  top: 5px;
}
.q-field--dense .q-field__control,
.q-field--dense .q-field__marginal {
  height: 34px;
}
</style>