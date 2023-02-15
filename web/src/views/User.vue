<!-- Copyright 2022 Zinc Labs Inc. and Contributors

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

     http:www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License. 
-->

<!-- eslint-disable vue/v-on-event-hyphenation -->
<!-- eslint-disable vue/attribute-hyphenation -->
<template>
  <q-page class="q-pa-none" style="min-height: inherit">
    <q-table
      ref="qTable"
      :rows="orgMembers"
      :columns="columns"
      row-key="id"
      :pagination="pagination"
      :filter="filterQuery"
      :filter-method="filterData"
    >
      <!-- 
      <template v-slot:body-cell-actions="props">
        <q-td :props="props">
          <q-btn
            v-if="props.row.email != store.state.userInfo.email"
            icon="perm_identity"
            class="iconHoverBtn"
            dense
            unelevated
            size="sm"
            color="blue-5"
            @click="updateUser(props)"
            :title="t('organization.invite')"
          ></q-btn>
        </q-td>
      </template>
       -->
      <template #no-data>
        <NoData></NoData>
      </template>
      <template #body-cell-role="props">
        <q-td
          :props="props"
          v-if="currentUserRole == 'admin' && !props.row.isLoggedinUser"
        >
          <q-select
            dense
            borderless
            v-model="props.row.role"
            :options="options"
            style="width: 70px"
            @update:model-value="
              updateUserRole(props.row.role, props.row.org_member_id)
            "
          />
        </q-td>
        <q-td :props="props" v-else>
          {{ props.row.role }}
        </q-td>
      </template>
      <template #body-cell-actions="props">
        <q-td :props="props" side>
          <q-btn
            v-if="props.row.email != store.state.userInfo.email"
            icon="img:/src/assets/images/common/view_icon.svg"
            :title="t('user.update')"
            class="iconHoverBtn"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="updateUser(props)"
          />

          <q-btn
            v-if="props.row.email == store.state.userInfo.email"
            icon="img:/src/assets/images/common/view_icon.svg"
            :title="t('user.update')"
            class="iconHoverBtn"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="addUser(props, false)"
          />
          <q-btn
            :title="t('user.updatenotallowed')"
            icon="img:/src/assets/images/common/view_icon.svg"
            flat
            size="sm"
            disable
            v-else
          />
          <q-btn
            v-if="props.row.email == `false_condition_to_hide_delete_button`"
            icon="img:/src/assets/images/common/delete_icon.svg"
            :title="t('user.delete')"
            class="q-ml-xs iconHoverBtn"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="deleteUser(props)"
          />
          <q-btn
            v-if="props.row.isLoggedinUser"
            icon="img:/src/assets/images/common/update_icon.svg"
            :title="t('user.update')"
            class="q-ml-xs iconHoverBtn"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="addUser(props, true)"
          />
        </q-td>
      </template>
      <template #top="scope">
        <div class="q-table__title full-width q-mb-md">
          {{ t("user.header") }}
        </div>
        <div class="full-width row q-mb-xs items-start">
          <q-input
            v-model="filterQuery"
            filled
            dense
            class="col-6 q-pr-sm"
            :placeholder="t('user.search')"
          >
            <template #prepend>
              <q-icon name="search" />
            </template>
          </q-input>

          <div
            class="col-5 q-pr-sm"
            v-if="
              currentUserRole == 'admin' && config.isZincObserveCloud == 'true'
            "
          >
            <div class="row invite-user">
              <q-input
                v-model="userEmail"
                class="col-9 q-pl-md"
                borderless
                dense
                :placeholder="t('user.inviteByEmail')"
              >
              </q-input>
              <q-separator vertical class="col-1 q-mx-md separator" />
              <div class="col-2 flex justify-center">
                <q-select
                  dense
                  borderless
                  v-model="selectedRole"
                  :options="options"
                  style="width: 70px"
                />
              </div>
            </div>
            <label class="inputHint">{{ t("user.inviteByEmailHint") }}</label>
            <q-btn
              v-if="currentUserRole == 'admin'"
              class="col-1 text-bold no-border"
              padding="sm 0"
              color="secondary"
              no-caps
              :label="t(`user.sendInvite`)"
              @click="inviteUser()"
              :disable="userEmail == ''"
            />
          </div>

          <div v-else class="col-6">
            <q-btn
              class="q-ml-md q-mb-xs text-bold no-border"
              style="float: right; cursor: pointer !important"
              padding="sm lg"
              color="secondary"
              no-caps
              icon="add"
              dense
              :label="t(`user.add`)"
              @click="addUser({}, false)"
            />
          </div>
        </div>

        <QTablePagination
          :scope="scope"
          :pageTitle="t('user.header')"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          position="top"
          @update:changeRecordPerPage="changePagination"
        />
      </template>

      <template #bottom="scope">
        <QTablePagination
          :scope="scope"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          position="bottom"
          @update:changeRecordPerPage="changePagination"
        />
      </template>
    </q-table>
    <q-dialog
      v-model="showUpdateUserDialog"
      position="right"
      full-height
      maximized
    >
      <update-user-role v-model="selectedUser" @updated="updateMember" />
    </q-dialog>

    <q-dialog
      v-model="showAddUserDialog"
      position="right"
      full-height
      maximized
    >
      <add-user
        style="width: 35vw"
        v-model="selectedUser"
        :isUpdated="isUpdated"
        @updated="addMember"
      />
    </q-dialog>

    <q-dialog v-model="confirmDelete">
      <q-card style="width: 240px">
        <q-card-section class="confirmBody">
          <div class="head">{{ t("user.confirmDeleteHead") }}</div>
          <div class="para">{{ t("user.confirmDeleteMsg") }}</div>
        </q-card-section>

        <q-card-actions class="confirmActions">
          <q-btn v-close-popup unelevated no-caps class="q-mr-sm">
            {{ t("user.cancel") }}
          </q-btn>
          <q-btn
            v-close-popup
            unelevated
            no-caps
            class="no-border"
            color="primary"
          >
            {{ t("user.ok") }}
          </q-btn>
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<script lang="ts">
import { defineComponent, onMounted, ref, watch } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, type QTableProps, date } from "quasar";
import { useI18n } from "vue-i18n";

import QTablePagination from "../components/shared/grid/Pagination.vue";
import usersService from "../services/users";
import UpdateUserRole from "../components/users/UpdateRole.vue"
import AddUser from "../components/users/add.vue"
import NoData from "../components/shared/grid/NoData.vue";
import { validateEmail } from "../utils/zincutils";
import config from "../aws-exports";
import organizationsService from "../services/organizations";

export default defineComponent({
  name: "PageUser",
  components: { QTablePagination, UpdateUserRole, NoData, AddUser },
  emits: [
    "updated:fields",
    "deleted:fields",
    "updated:dates",
    "update:changeRecordPerPage",
    "update:maxRecordToReturn",
  ],
  setup(props, { emit }) {
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const orgMembers: any = ref([]);
    const resultTotal = ref<number>(0);
    const showUpdateUserDialog: any = ref(false);
    const showAddUserDialog: any = ref(false);
    const confirmDelete = ref<boolean>(false);
    const selectedUser: any = ref({});
    const orgData: any = ref(store.state.selectedOrganization);
    const isUpdated: any = ref(false);
    const qTable: any = ref(null);
    const columns: any = ref<QTableProps["columns"]>([
      {
        name: "#",
        label: "#",
        field: "#",
        align: "left",
      },
      {
        name: "email",
        field: "email",
        label: t("user.email"),
        align: "left",
        sortable: true,
      },
      {
        name: "first_name",
        field: "first_name",
        label: t("user.firstName"),
        align: "left",
        sortable: true,
      },
      {
        name: "last_name",
        field: "last_name",
        label: t("user.lastName"),
        align: "left",
        sortable: true,
      },
      {
        name: "role",
        field: "role",
        label: t("user.role"),
        align: "left",
        sortable: true,
      },
      {
        name: "member_created",
        field: "member_created",
        label: t("user.memberCreated"),
        align: "left",
        sortable: true,
      },
    ]);
    const userEmail: any = ref('');
    const options = [t("user.admin"), t("user.member")]
    const selectedRole = ref(options[0]);
    const currentUserRole = ref('')

    const getOrgMembers = (orgId: string) => {
      if (orgId != "") {
        const dismiss = $q.notify({
          spinner: true,
          message: "Please wait while loading users...",
        });

        usersService.orgUsers(0, 1000, "email", false, "", store.state.selectedOrganization.identifier).then((res) => {
          resultTotal.value = res.data.data.length;
          columns.value = columns.value.filter((v: any) => v.name !== "actions");
          let counter = 1;
          orgMembers.value = res.data.data.map((data: any) => {
            if (store.state.userInfo.email == data.email) {
              currentUserRole.value = data.role
            }
            if (store.state.userInfo.email == data.email && data.role == "admin") {
              columns.value.push({
                name: "actions",
                field: "actions",
                label: t("user.actions"),
                align: "center",
              });

            }
            return {
              "#": counter <= 9 ? `0${counter++}` : counter++,
              email: data.email,
              first_name: data.first_name,
              last_name: data.last_name,
              role: data.role,
              member_created: date.formatDate(parseInt(data.member_created), "YYYY-MM-DDTHH:mm:ssZ"),
              member_updated: date.formatDate(parseInt(data.member_updated), "YYYY-MM-DDTHH:mm:ssZ"),
              org_member_id: data.org_member_id,
              isLoggedinUser: store.state.userInfo.email == data.email
            };
          });

          dismiss();
        });
      }
    };

    if (orgMembers.value.length == 0) {
      getOrgMembers(store.state.selectedOrganization.identifier);
    }

    interface OptionType {
      label: String;
      value: number | String;
    }
    const perPageOptions: any = [
      { label: "5", value: 5 },
      { label: "10", value: 10 },
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "All", value: 0 },
    ];
    const maxRecordToReturn = ref<number>(100);
    const selectedPerPage = ref<number>(20);
    const pagination: any = ref({
      rowsPerPage: 20,
    });
    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);
    };
    const changeMaxRecordToReturn = (val: any) => {
      maxRecordToReturn.value = val;
    };

    const updateUser = (props: any) => {
      selectedUser.value = props.row;
      showUpdateUserDialog.value = true;
    }

    const addUser = (props: any, is_updated: boolean) => {
      isUpdated.value = is_updated;
      if (props.row != undefined) {
        selectedUser.value = props.row;  
      } else {
        selectedUser.value = {};
      }
      
      showAddUserDialog.value = true;
    }

    const updateMember = (data: any) => {
      if (data.data != undefined) {
        orgMembers.value.forEach((member: any, key: number) => {
          if (member.org_member_id == data.data.id) {
            orgMembers.value[key].role = data.data.role;
          }
        });
        showUpdateUserDialog.value = false;
      }
    }

    const addMember = (data: any) => {
      alert("addMember")
      if (data.data != undefined) {
        alert(JSON.stringify(data.data))
        orgMembers.value.push({
          "#": orgMembers.value.length + 1,
          email: data.data.email,
          first_name: data.data.first_name,
          last_name: data.data.last_name,
          role: data.data.role,
        });
        showAddUserDialog.value = false;
      }
    }

    const deleteUser = (props: any) => {
      confirmDelete.value = true;
    };

    const inviteUser = () => {      
      const emailArray = userEmail.value.split(';').filter((email: any) => email).map((email: any) => email.trim())
      const validationArray = emailArray.map((email: any) => validateEmail(email))
      
      if (!validationArray.includes(false)) {
        const dismiss = $q.notify({
          spinner: true,
          message: "Please wait...",
          timeout: 2000,
        });

        organizationsService.add_members(
          { member_lists: emailArray, role: selectedRole.value },
          store.state.selectedOrganization.identifier
        )
          .then((res: { data: any }) => {
            const data = res.data;

            if (data.data.invalid_members != null) {
              const message = `Error while member invitation: ${data.data.invalid_members.toString()}`;

              $q.notify({
                type: "negative",
                message: message,
                timeout: 15000,
              });
            } else {
              $q.notify({
                type: "positive",
                message: data.message,
                timeout: 5000,
              });
            }

            // this.$emit("updated");
            dismiss();
          }).catch(error => {
            dismiss();
            $q.notify({
                type: "negative",
                message: error.message,
                timeout: 5000,
              });
          })

        userEmail.value = "";
      } else {
        $q.notify({
          type: "negative",
          message: `Please enter correct email id.`,
        });
      }
    }
    const updateUserRole = (userRole: any, orgMemberId: any) => {

      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });

      organizationsService.update_member_role(
        {
          id: parseInt(orgMemberId),
          role: userRole,
          organization_id: parseInt(store.state.selectedOrganization.id),
        },
        store.state.selectedOrganization.identifier
      ).then((res: { data: any }) => {
        if (res.data.error_members != null) {
          const message = `Error while updating organization member`;
          $q.notify({
            type: "negative",
            message: message,
            timeout: 15000,
          });
        } else {
          $q.notify({
            type: "positive",
            message: "Organization member updated successfully.",
            timeout: 3000,
          });
        }
        dismiss();
      }).catch(error => {
        dismiss();
        console.log(error);
      });
    }
    return {
      t,
      qTable,
      router,
      store,
      config,
      orgMembers,
      columns,
      orgData,
      confirmDelete,
      deleteUser,
      getOrgMembers,
      updateUser,
      updateMember,
      addUser,
      addMember,
      isUpdated,
      showAddUserDialog,
      pagination,
      resultTotal,
      selectedUser,
      perPageOptions,
      selectedPerPage,
      changePagination,
      maxRecordToReturn,
      showUpdateUserDialog,
      changeMaxRecordToReturn,
      filterQuery: ref(""),
      filterData(rows: any, terms: any) {
        var filtered = [];
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (rows[i]["first_name"]?.toLowerCase().includes(terms) || rows[i]["last_name"]?.toLowerCase().includes(terms) || rows[i]["email"]?.toLowerCase().includes(terms) || rows[i]["role"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
        return filtered;
      },
      userEmail,
      selectedRole,
      options,
      inviteUser,
      currentUserRole,
      updateUserRole
    };
  },
  computed: {
    selectedOrg() {
      return this.store.state.selectedOrganization.identifier
    }
  },
  watch: {
    selectedOrg(newVal: any, oldVal: any) {
      this.orgData = newVal;
      if ((newVal != oldVal || this.orgMembers.value == undefined) && this.router.currentRoute.value.name == "users") {
        this.getOrgMembers(this.store.state.selectedOrganization.identifier);
      }
    }
  }
});
</script>

<style lang="scss" scoped>
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}

.confirmBody {
  padding: 11px 1.375rem 0;
  font-size: 0.875rem;
  text-align: center;
  font-weight: 700;

  .head {
    line-height: 2.125rem;
    margin-bottom: 0.5rem;
    color: $dark-page;
  }
  .para {
    color: $light-text;
  }
}
.confirmActions {
  justify-content: center;
  padding: 1.25rem 1.375rem 1.625rem;
  display: flex;

  .q-btn {
    font-size: 0.75rem;
    font-weight: 700;
  }
}

.non-selectable {
  cursor: default !important;
}

.invite-user {
  background: $input-bg;
  border-radius: 4px;

  .separator {
    width: 1px;
  }
}

.inputHint {
  font-size: 11px;
  color: $light-text;
}
</style>
